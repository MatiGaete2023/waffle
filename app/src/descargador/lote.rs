//! Ejecución secuencial de un lote Lite (`lotes.BatchRunner.run` y `motor.run_sequence` de Flow Python).
//! Una carpeta por lote, nombres descriptivos, manifiesto por consulta y `resumen.json` actualizado tras cada paso.
//! Solo lectura: el puente admite órdenes de consulta y descarga; nada se escribe en RUS ni SITFA.

use super::evidencia::png_a_pdf;
use super::sitfa::{
    siguiente_consulta, validar_descarga, validar_pantalla_evidencia, validar_respuesta, Consulta,
    Pares, Respuesta,
};
use super::xls::{huella, Verificador};
use super::{Alcance, Catalogo, Edicion, Pantalla, Perfil, PlanLote, Puente, Seleccion};
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::NaiveDate;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, LazyLock};
use std::time::{Duration, Instant};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

pub const PLAZO: Duration = Duration::from_secs(75);
const PLAZO_CIERRE: Duration = Duration::from_secs(8);
const ESPERA_SITFA_MS: u64 = 60_000;
const MAX_NOMBRE: usize = 150;
const SUFIJO_MAXIMO: &str = " (99) - p99 de 99.xls";
const CANCELADO: &str = "Lote cancelado. Se conservan los archivos verificados.";
const CANCELADO_PAGINAS: &str = "Descarga cancelada. Las paginas ya verificadas se conservan.";

/// Orden local hacia la extensión. `Puente` la implementa; las pruebas usan un SITFA ficticio.
pub trait Conexion {
    /// Los motores que envuelven un puente pueden ejecutar un lote en varias pestañas independientes.
    fn ejecutar_distribuido(
        &self, _solicitud: &Solicitud<'_>, _emitir: &mut dyn FnMut(Evento),
    ) -> Option<Result<Vec<Value>, String>> {
        None
    }

    fn llamar(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, String>;
    /// E-Q3: el puente real solo acepta las lecturas de contexto tras verificar la extensión Integral.
    fn habilitar_lecturas(&self, _integral: bool) {}
    /// V-PAG-01: la extensión Integral >= 2.11.0 anuncia `search_and_download`.
    fn soporta_search_and_download(&self) -> bool {
        false
    }
}

impl Conexion for Puente {
    fn llamar(&self, comando: &str, payload: Value, plazo: Duration) -> Result<Value, String> {
        Puente::llamar(self, comando, payload, plazo)
    }

    fn habilitar_lecturas(&self, integral: bool) {
        Puente::habilitar_lecturas(self, integral)
    }

    fn soporta_search_and_download(&self) -> bool {
        Puente::soporta_search_and_download(self)
    }
}

pub struct Solicitud<'a> {
    pub catalogo: &'a Catalogo,
    pub edicion: Edicion,
    pub plan: &'a PlanLote,
    pub destino: &'a Path,
    pub opciones: &'a Opciones,
    pub cancelar: &'a AtomicBool,
}

#[derive(Clone, Debug)]
pub struct Opciones {
    /// Conserva los filtros adicionales del formulario SITFA (`keep_filters`).
    pub conservar_filtros: bool,
    /// Pantallazo PDF de las consultas sin registros.
    pub evidencia: bool,
    pub pausa: Duration,
}

impl Default for Opciones {
    fn default() -> Self {
        Self {
            conservar_filtros: false,
            evidencia: true,
            pausa: Duration::ZERO,
        }
    }
}

/// Regla de `descargador.py::batch`: los filtros privados solo se conservan en Seguimiento y Litigantes.
/// La usa `iniciar` (interfaz) y la prueba D-2; en calendarios y Carga el valor pedido no se conserva.
pub fn conservar_filtros(pedido: bool, pantalla: Pantalla) -> bool {
    pedido && matches!(pantalla, Pantalla::Seguimiento | Pantalla::Litigantes)
}

#[derive(Clone, Debug, PartialEq)]
pub enum Evento {
    Carpeta(PathBuf),
    Plan(usize),
    Consulta { indice: usize, total: usize, etiqueta: String },
    Estado(String),
    Total(usize),
    Pagina { consulta: String, pagina: usize, total: usize },
    Vacia { consulta: String },
    Evidencia { archivo: String },
    ConsultaLista { consulta: String, estado: String },
    Aviso(String),
    Fin(Fin),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fin {
    pub carpeta: PathBuf,
    pub consultas: usize,
    pub archivos: usize,
    pub pdfs: usize,
    pub vacias: usize,
    pub registros: usize,
}

const MODALIDADES_LITE: [(&str, &str); 4] =
    [("1", "Residencia"), ("2", "Ambulatorio"), ("3", "FAE"), ("4", "DCE")];

fn titulo(pantalla: Pantalla) -> &'static str {
    match pantalla {
        Pantalla::Seguimiento => "Seguimiento",
        Pantalla::Litigantes => "Busqueda Lit",
        Pantalla::CalendarioInformes => "Informes por vencer",
        Pantalla::CalendarioMedidas => "Medidas por vencer",
        Pantalla::Carga => "Carga",
    }
}

fn largo(s: &str) -> usize {
    s.chars().count()
}

fn cortar(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

/// `lotes.safe_name`: sin tildes, caracteres prohibidos en Windows ni puntos; espacios simples; 125 caracteres.
pub fn nombre_seguro(valor: &str) -> String {
    let limpio: String = valor
        .nfkd()
        .filter(|c| !is_combining_mark(*c))
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '.') || (c as u32) < 0x20 {
                ' '
            } else {
                c
            }
        })
        .collect();
    let unido = limpio.split_whitespace().collect::<Vec<_>>().join(" ");
    cortar(unido.trim_end_matches([' ', '.']), 125)
}

/// `lotes.tribunal_name`: abreviaturas de los juzgados.
pub fn nombre_tribunal(valor: &str) -> String {
    static REGLAS: LazyLock<[(Regex, &str); 3]> = LazyLock::new(|| {
        [
            (Regex::new(r"(?i)^Juzgado de Familia\s+").unwrap(), "Jdo Familia "),
            (Regex::new(r"(?i)^Jgdo\s+L\s+y\s+G\s+de\s+").unwrap(), "Jdo Letras y Garantia "),
            (Regex::new(r"(?i)^Jgdo\s+(?:Letras|L)\s+de\s+").unwrap(), "Jdo Letras "),
        ]
    });
    let mut v = nombre_seguro(valor);
    for (re, reemplazo) in REGLAS.iter() {
        v = re.replacen(&v, 1, *reemplazo).into_owned();
    }
    v
}

pub(super) fn fecha_sitfa(d: NaiveDate) -> String {
    d.format("%d/%m/%Y").to_string()
}

fn etiqueta_fechas(fechas: (NaiveDate, NaiveDate)) -> String {
    format!("{} al {}", fechas.0.format("%d-%m-%Y"), fechas.1.format("%d-%m-%Y"))
}

fn calendario(p: Pantalla) -> bool {
    matches!(p, Pantalla::CalendarioInformes | Pantalla::CalendarioMedidas)
}

fn etiqueta<'a>(grupo: &'a std::collections::BTreeMap<String, String>, clave: &str) -> &'a str {
    grupo.get(clave).map(String::as_str).unwrap_or("")
}

/// `lotes.report_label`.
fn etiqueta_reporte(perfil: &Perfil, catalogo: &Catalogo) -> String {
    match &perfil.alcance {
        Alcance::Seguimiento { pestana, reporte, .. } if pestana == "Informes" => {
            etiqueta(&catalogo.informes, reporte.as_deref().unwrap_or("")).into()
        }
        Alcance::Seguimiento { pestana, reporte, .. } if pestana == "Cumplimiento" => match reporte {
            Some(r) => etiqueta(&catalogo.medidas, r).into(),
            None => "Todas las medidas".into(),
        },
        _ => String::new(),
    }
}

/// `lotes.query_parts`: etiquetas en el orden en que aparecen en el nombre del archivo.
pub fn partes_consulta(perfil: &Perfil, catalogo: &Catalogo, fechas: Option<(NaiveDate, NaiveDate)>) -> Vec<String> {
    let mut partes = vec![titulo(perfil.pantalla).to_string()];
    match &perfil.alcance {
        Alcance::Seguimiento { pestana, modalidad, .. } => {
            partes.push(pestana.clone());
            partes.push(
                MODALIDADES_LITE
                    .iter()
                    .find(|(c, _)| c == modalidad)
                    .map(|(_, n)| n.to_string())
                    .unwrap_or_else(|| etiqueta(&catalogo.modalidades, modalidad).into()),
            );
            let reporte = etiqueta_reporte(perfil, catalogo);
            if !reporte.is_empty() {
                partes.push(reporte);
            }
        }
        Alcance::Litigantes { estado } => partes.push(etiqueta(&catalogo.estados, estado).into()),
        Alcance::Calendario { mes, anio } => partes.push(format!(
            "{} {}",
            etiqueta(&catalogo.meses, mes),
            etiqueta(&catalogo.anios, anio)
        )),
        Alcance::Carga { .. } => {}
    }
    if let Some(f) = fechas.filter(|_| !calendario(perfil.pantalla)) {
        partes.push(etiqueta_fechas(f));
    }
    partes
        .iter()
        .map(|p| nombre_seguro(p))
        .filter(|p| !p.is_empty())
        .collect()
}

/// `lotes.name_limit`: carpeta, nombre y sufijo de página caben en los 260 caracteres de Windows.
pub fn limite_nombre(carpeta: &Path) -> usize {
    let absoluta = std::path::absolute(carpeta).unwrap_or_else(|_| carpeta.to_path_buf());
    let disponible = 259usize
        .saturating_sub(largo(&absoluta.to_string_lossy()))
        .saturating_sub(1 + largo(SUFIJO_MAXIMO));
    disponible.clamp(60, MAX_NOMBRE)
}

/// `lotes.file_base`. Ej.: «Seguimiento - Cumplimiento - Ambulatorio - Vencidos - Jdo Familia X - 01-10-2026 al 31-10-2026».
pub fn base_archivo(
    perfil: &Perfil,
    catalogo: &Catalogo,
    fechas: Option<(NaiveDate, NaiveDate)>,
    limite: usize,
) -> String {
    let partes = partes_consulta(perfil, catalogo, fechas);
    let corte = (if perfil.pantalla == Pantalla::Seguimiento { 4 } else { 2 }).min(partes.len());
    let cabeza = partes[..corte].join(" - ");
    let cola = &partes[corte..];
    let tribunal = nombre_tribunal(etiqueta(&catalogo.tribunales, &perfil.tribunal));
    let espacio = limite as isize
        - largo(&cabeza) as isize
        - cola.iter().map(|x| largo(x) as isize + 3).sum::<isize>()
        - 3;
    let tribunal = cortar(&tribunal, espacio.max(12) as usize);
    let tribunal = tribunal.trim_end_matches([' ', '.']);
    let mut todo = vec![cabeza.as_str(), tribunal];
    todo.extend(cola.iter().map(String::as_str));
    cortar(&todo.join(" - "), limite)
        .trim_end_matches([' ', '.', '-'])
        .to_string()
}

/// `descargador/lotes.file_base` (Integral): «ESP Ambulatorio Jdo Familia X», «Informes Jdo Familia X»…
pub fn base_integral(perfil: &Perfil, catalogo: &Catalogo) -> String {
    let tribunal = nombre_tribunal(etiqueta(&catalogo.tribunales, &perfil.tribunal));
    let base = match (&perfil.alcance, perfil.pantalla) {
        (_, Pantalla::CalendarioInformes) => format!("Informes {tribunal}"),
        (_, Pantalla::CalendarioMedidas) => format!("Medidas por vencer {tribunal}"),
        (Alcance::Litigantes { estado }, _) => format!("Litigantes {} {tribunal}", etiqueta(&catalogo.estados, estado)),
        (Alcance::Carga { desde, hasta }, _) => format!("Carga {tribunal} {} {}", fecha_sitfa(*desde), fecha_sitfa(*hasta)),
        (Alcance::Seguimiento { pestana, modalidad, .. }, _) => match pestana.as_str() {
            "Informes" => format!("Informes {tribunal}"),
            "Egresados" => format!("Egresados {tribunal}"),
            p => format!(
                "{} {} {tribunal}",
                if p == "Espera" { "ESP" } else { "CUMP" },
                MODALIDADES_LITE.iter().find(|(c, _)| c == modalidad).map_or("", |(_, n)| n)
            ),
        },
        (Alcance::Calendario { .. }, _) => tribunal,
    };
    nombre_seguro(&base)
}

/// `perfiles.selection_profile`: clave y etiqueta del manifiesto.
fn caso_y_rotulo(perfil: &Perfil, catalogo: &Catalogo) -> (String, String) {
    let t = &perfil.tribunal;
    let nombre = etiqueta(&catalogo.tribunales, t);
    match &perfil.alcance {
        Alcance::Seguimiento { pestana, modalidad, reporte } => {
            let mut clave = format!("T{t}-{pestana}-M{modalidad}");
            let mut rotulo = format!("{nombre} / {pestana} / {}", etiqueta(&catalogo.modalidades, modalidad));
            if let Some(r) = reporte.as_ref().filter(|_| matches!(pestana.as_str(), "Informes" | "Cumplimiento")) {
                let grupo = if pestana == "Informes" { &catalogo.informes } else { &catalogo.medidas };
                rotulo.push_str(&format!(" / {}", etiqueta(grupo, r)));
                clave.push_str(&format!("-I{r}"));
            }
            (clave, rotulo)
        }
        Alcance::Litigantes { estado } => (
            format!("T{t}-Litigantes-E{estado}"),
            format!("{nombre} / Litigantes / {}", etiqueta(&catalogo.estados, estado)),
        ),
        Alcance::Calendario { mes, anio } => {
            let tipo = if perfil.pantalla == Pantalla::CalendarioInformes { "Informes" } else { "Medidas" };
            (format!("T{t}-{tipo}-{anio}-{mes}"), format!("{nombre} / {tipo} por vencer / {mes}-{anio}"))
        }
        Alcance::Carga { desde, hasta } => (
            format!("T{t}-Carga-{}-{}", desde.format("%Y-%m-%d"), hasta.format("%Y-%m-%d")),
            format!("{nombre} / Carga / {}–{}", fecha_sitfa(*desde), fecha_sitfa(*hasta)),
        ),
    }
}

/// `asdict(Selection)` de Integral: agrega `start` y `end`, vacíos fuera de Carga.
pub(super) fn seleccion_edicion(perfil: &Perfil, edicion: Edicion) -> Value {
    let mut v = seleccion(perfil);
    if edicion == Edicion::Integral && perfil.pantalla != Pantalla::Carga {
        v["start"] = Value::Null;
        v["end"] = Value::Null;
    }
    v
}

/// `dataclasses.asdict(Selection)` con los valores por omisión de Flow Python.
fn seleccion(perfil: &Perfil) -> Value {
    let pantalla = serde_json::to_value(perfil.pantalla).unwrap_or_default();
    match &perfil.alcance {
        Alcance::Seguimiento { pestana, modalidad, reporte } => json!({
            "tribunal": perfil.tribunal, "tab": pestana, "modality": modalidad, "report": reporte,
            "screen": pantalla, "state": "1", "month": null, "year": null,
        }),
        Alcance::Litigantes { estado } => json!({
            "tribunal": perfil.tribunal, "tab": "", "modality": "", "report": null,
            "screen": pantalla, "state": estado, "month": null, "year": null,
        }),
        Alcance::Calendario { mes, anio } => json!({
            "tribunal": perfil.tribunal, "tab": "", "modality": "", "report": null,
            "screen": pantalla, "state": "1", "month": mes, "year": anio,
        }),
        Alcance::Carga { desde, hasta } => json!({
            "tribunal": perfil.tribunal, "tab": "", "modality": "", "report": null,
            "screen": pantalla, "state": "1", "month": null, "year": null,
            "start": fecha_sitfa(*desde), "end": fecha_sitfa(*hasta),
        }),
    }
}

/// `lotes.evidence_context`: identifica la consulta en el pantallazo.
fn contexto(perfil: &Perfil, catalogo: &Catalogo, fechas: Option<(NaiveDate, NaiveDate)>) -> Vec<[String; 2]> {
    let par = |a: &str, b: String| [a.to_string(), b];
    let mut c = vec![
        par("Tribunal", format!("{} (código {})", etiqueta(&catalogo.tribunales, &perfil.tribunal), perfil.tribunal)),
        par("Consulta", titulo(perfil.pantalla).into()),
    ];
    match &perfil.alcance {
        Alcance::Seguimiento { pestana, modalidad, .. } => {
            c.push(par("Pestaña", pestana.clone()));
            c.push(par("Modalidad", etiqueta(&catalogo.modalidades, modalidad).into()));
            let reporte = etiqueta_reporte(perfil, catalogo);
            if !reporte.is_empty() {
                c.push(par(if pestana == "Informes" { "Informe" } else { "Medidas" }, reporte));
            }
        }
        Alcance::Litigantes { estado } => c.push(par("Estado", etiqueta(&catalogo.estados, estado).into())),
        Alcance::Calendario { mes, anio } => c.push(par(
            "Mes",
            format!("{} {}", etiqueta(&catalogo.meses, mes), etiqueta(&catalogo.anios, anio)),
        )),
        Alcance::Carga { .. } => {}
    }
    if !calendario(perfil.pantalla) {
        c.push(par(
            "Fechas",
            fechas
                .map(|(a, b)| format!("{} – {}", fecha_sitfa(a), fecha_sitfa(b)))
                .unwrap_or_else(|| "Sin filtro de fechas".into()),
        ));
    }
    for p in &mut c {
        p[1] = cortar(&p[1], 500);
    }
    c
}

/// Contexto del pantallazo en `descargador/lotes.py`: Tribunal, Modalidad y Pestaña, Consulta (etiqueta) y Fechas.
fn contexto_integral(perfil: &Perfil, catalogo: &Catalogo, fechas: Option<(NaiveDate, NaiveDate)>) -> Vec<[String; 2]> {
    let par = |a: &str, b: String| [a.to_string(), b];
    let mut c = vec![par(
        "Tribunal",
        format!("{} (código {})", etiqueta(&catalogo.tribunales, &perfil.tribunal), perfil.tribunal),
    )];
    if let Alcance::Seguimiento { pestana, modalidad, .. } = &perfil.alcance {
        c.push(par("Modalidad", etiqueta(&catalogo.modalidades, modalidad).into()));
        c.push(par("Pestaña", pestana.clone()));
    }
    c.push(par("Consulta", caso_y_rotulo(perfil, catalogo).1));
    c.push(par(
        "Fechas",
        fechas
            .map(|(a, b)| format!("{} – {}", fecha_sitfa(a), fecha_sitfa(b)))
            .unwrap_or_else(|| "Sin filtro de fechas".into()),
    ));
    c
}

pub(super) fn ahora() -> String {
    chrono::Local::now().to_rfc3339_opts(chrono::SecondsFormat::Micros, false)
}

/// `motor.write_error`: motivo concreto de una escritura fallida.
fn error_escritura(e: &std::io::Error, ruta: &Path) -> String {
    let completa = std::path::absolute(ruta).unwrap_or_else(|_| ruta.to_path_buf());
    let codigo = e.raw_os_error();
    let carpeta = completa.parent().map(|p| p.display().to_string()).unwrap_or_default();
    let motivo = if matches!(codigo, Some(3 | 206)) || largo(&completa.to_string_lossy()) >= 260 {
        format!(
            "la ruta es demasiado larga para Windows ({} caracteres). Elige una carpeta de destino más corta",
            largo(&completa.to_string_lossy())
        )
    } else if e.kind() == ErrorKind::PermissionDenied || matches!(codigo, Some(5 | 13 | 32 | 33)) {
        "Windows no permitió escribir en esa carpeta (permiso, archivo abierto o protección «Acceso controlado a carpetas» del antivirus). Permite Flow en esa protección o elige otra carpeta".into()
    } else if e.kind() == ErrorKind::StorageFull || matches!(codigo, Some(28 | 112)) {
        "no queda espacio en el disco".into()
    } else {
        format!("error del sistema {:?} {}", e.kind(), codigo.unwrap_or_default())
    };
    format!("Falló la escritura local del archivo: {motivo}. Carpeta: {carpeta}")
}

/// `motor.save_exclusive`: nunca sobrescribe; solo queda el archivo escrito completo.
pub fn guardar_exclusivo(ruta: &Path, datos: &[u8]) -> Result<(), String> {
    let mut archivo = match OpenOptions::new().write(true).create_new(true).open(ruta) {
        Ok(a) => a,
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            return Err("Ya existe el archivo de destino; usar una ejecucion nueva.".into())
        }
        Err(e) => return Err(error_escritura(&e, ruta)),
    };
    if let Err(e) = archivo.write_all(datos).and_then(|_| archivo.sync_all()) {
        drop(archivo);
        let _ = fs::remove_file(ruta);
        return Err(error_escritura(&e, ruta));
    }
    Ok(())
}

/// Escritura atómica: temporal y reemplazo. `resumen.json` usa `resumen.tmp`; los manifiestos, `<nombre>.tmp`.
pub(super) fn escribir_json(carpeta: &Path, nombre: &str, valor: &Value) -> Result<(), String> {
    let temporal = match nombre.strip_suffix(".json").filter(|_| nombre == "resumen.json") {
        Some(raiz) => carpeta.join(format!("{raiz}.tmp")),
        None => carpeta.join(format!("{nombre}.tmp")),
    };
    let datos = serde_json::to_string_pretty(valor).map_err(|_| "No se pudo preparar el resumen del lote.")?;
    fs::write(&temporal, datos)
        .and_then(|_| fs::rename(&temporal, carpeta.join(nombre)))
        .map_err(|e| error_escritura(&e, &carpeta.join(nombre)))
}

pub(super) fn cuerpo(respuesta: &Value, excel: bool) -> Result<Vec<u8>, String> {
    let error = || "Respuesta vacía, vencida o fuera del formato permitido.".to_string();
    let texto = respuesta["body"].as_str().ok_or_else(error)?;
    if texto.len() > 4 * (80 * 1024 * 1024 + 2) / 3 {
        return Err(error());
    }
    if !excel && !respuesta["type"].is_string() {
        return Err(error());
    }
    let datos = STANDARD.decode(texto).map_err(|_| error())?;
    if respuesta["status"].as_u64() != Some(200) || datos.is_empty() || datos.len() > 80 * 1024 * 1024 {
        return Err(error());
    }
    if !excel
        && !respuesta["type"]
            .as_str()
            .is_some_and(|t| t.to_ascii_lowercase().contains("html"))
    {
        return Err(error());
    }
    Ok(datos)
}

fn cancelado(cancelar: &AtomicBool, mensaje: &str) -> Result<(), String> {
    if cancelar.load(Ordering::SeqCst) {
        Err(mensaje.into())
    } else {
        Ok(())
    }
}

/// Espera cancelable, como `PauseGate.wait`.
fn pausa(duracion: Duration, cancelar: &AtomicBool, mensaje: &str) -> Result<(), String> {
    let mut resta = duracion;
    while !resta.is_zero() {
        cancelado(cancelar, mensaje)?;
        let paso = resta.min(Duration::from_millis(50));
        std::thread::sleep(paso);
        resta -= paso;
    }
    cancelado(cancelar, mensaje)
}

pub(super) fn carpeta_nueva(destino: &Path) -> Result<PathBuf, String> {
    let ahora = chrono::Local::now();
    let mut carpeta = destino.join(ahora.format("%Y-%m-%d %H-%M-%S").to_string());
    if carpeta.exists() {
        carpeta = destino.join(ahora.format("%Y-%m-%d %H-%M-%S-%6f").to_string());
    }
    fs::create_dir_all(destino).map_err(|e| error_escritura(&e, destino))?;
    fs::create_dir(&carpeta).map_err(|e| {
        if e.kind() == ErrorKind::AlreadyExists {
            "Ya existe la carpeta del lote; vuelve a iniciar.".to_string()
        } else {
            error_escritura(&e, &carpeta)
        }
    })?;
    Ok(carpeta)
}

struct Lote<'a> {
    conexion: &'a dyn Conexion,
    edicion: Edicion,
    catalogo: &'a Catalogo,
    opciones: &'a Opciones,
    cancelar: &'a AtomicBool,
    carpeta: PathBuf,
}

fn escribir_manifiesto(
    carpeta: &Path,
    nombre: &str,
    caso: &(String, String),
    archivos: &[Value],
    total: usize,
    estado: &str,
) -> Result<(), String> {
    escribir_json(
        carpeta,
        nombre,
        &json!({
            "fecha_hora": ahora(), "caso": caso.0, "consulta": caso.1, "estado": estado,
            "paginas_esperadas": total, "paginas_verificadas": archivos.len(), "archivos": archivos,
        }),
    )
}

fn extraer_excel_compuesto(
    resp: &Value,
    pagina: &super::sitfa::Pagina,
    plantilla_primera: &str,
    lote: &Lote<'_>,
) -> Result<Vec<u8>, String> {
    if resp["excel"].is_object() {
        if let Some(url) = resp["excel"]["url"].as_str() {
            validar_descarga(url, plantilla_primera)?;
            if url != pagina.excel_url {
                return Err("El enlace Excel no coincide con la consulta actual.".into());
            }
        }
        cuerpo(&resp["excel"], true)
    } else {
        cuerpo(
            &lote.pedir("download", json!({"url": pagina.excel_url, "timeout": ESPERA_SITFA_MS}))?,
            true,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn procesar_pagina_descargada(
    carpeta: &Path,
    verificador: &mut Verificador,
    resultados: &mut Vec<Value>,
    consulta: &Consulta,
    pagina: &super::sitfa::Pagina,
    numero: usize,
    total: usize,
    archivo: String,
    datos: &[u8],
    html_actual: &[u8],
    listado: Option<&super::vinculos::Seleccion>,
    caso: &(String, String),
    manifiesto: &str,
) -> Result<Option<Vec<String>>, String> {
    let (libro, digest) = verificador.verificar(datos, pagina, consulta)?;
    let columnas_xls = (numero == 1 && consulta.pantalla == Pantalla::Seguimiento)
        .then(|| super::sitfa::registros::cabeceras_desde_grilla(libro.grilla()));
    if Path::new(&archivo).file_name().and_then(|n| n.to_str()) != Some(archivo.as_str())
        || !archivo.ends_with(".xls")
    {
        return Err("Nombre de archivo fuera de alcance.".into());
    }
    let ruta = carpeta.join(&archivo);
    guardar_exclusivo(&ruta, datos)?;
    let guardado = fs::read(&ruta).map_err(|e| error_escritura(&e, &ruta))?;
    if guardado != datos {
        return Err("El archivo guardado no coincide con la descarga validada.".into());
    }
    verificador.registrar(&libro, digest.clone())?;
    resultados.push(json!({
        "pagina": numero, "total": total, "archivo": archivo, "bytes": guardado.len(),
        "sha256": digest, "formato": libro.formato,
        "registros": if consulta.pantalla == Pantalla::Carga {
            pagina.carga.values().sum::<usize>()
        } else {
            libro.registros.values().sum::<usize>()
        },
        "coincide_con_html": true, "estado": "OK",
    }));
    // `motor.run_sequence`: en Seguimiento cada página conserva los vínculos de ingreso comprobados del
    // listado (`search_page.bindings` → `vinculos_ingreso`), que el paquete usa para la identidad remota.
    if let Some(seleccion) = listado {
        let vinculos: Vec<Value> = super::vinculos::extraer(html_actual, seleccion)
            .map(|v| v.iter().filter(|v| v.bitacora_verificada).map(super::vinculos::Vinculo::bitacora_python).collect())
            .unwrap_or_default();
        if !vinculos.is_empty() {
            resultados.last_mut().unwrap()["vinculos_ingreso"] = json!(vinculos);
        }
    }
    escribir_manifiesto(carpeta, manifiesto, caso, resultados, total, "EN_CURSO")?;
    Ok(columnas_xls)
}

impl Lote<'_> {
    fn pedir(&self, comando: &str, payload: Value) -> Result<Value, String> {
        self.conexion.llamar(comando, payload, PLAZO)
    }

    fn manifiesto(&self, nombre: &str, caso: &(String, String), archivos: &[Value], total: usize, estado: &str) -> Result<(), String> {
        escribir_manifiesto(&self.carpeta, nombre, caso, archivos, total, estado)
    }

    /// `motor.run_sequence`: `search_and_download` + pipelining local cuando la extensión es >= 2.11.0,
    /// o `POST(n) → XLS(n) → validar → fsync → releer → POST(n+1)` en modo legado. Sin reintentos silenciosos.
    #[allow(clippy::too_many_arguments)]
    fn paginas(
        &self,
        pares: &Pares,
        primera: super::sitfa::Pagina,
        consulta: &Consulta,
        caso: &(String, String),
        manifiesto: &str,
        nombre: &dyn Fn(usize, usize) -> String,
        emitir: &mut dyn FnMut(Evento),
        etiqueta_consulta: &str,
        html_primera: &[u8],
        datos_primera: Option<Vec<u8>>,
        latencia_primera: Duration,
        listado: Option<&super::vinculos::Seleccion>,
    ) -> Result<(Vec<Value>, Option<Vec<String>>), String> {
        let total = primera.total;
        let mut resultados: Vec<Value> = Vec::new();
        let mut columnas_xls_p1: Option<Vec<String>> = None;
        self.manifiesto(manifiesto, caso, &resultados, total, "EN_CURSO")?;
        let usa_compuesto = self.conexion.soporta_search_and_download();
        let ejecucion = if usa_compuesto {
            let worker_fallo = AtomicBool::new(false);
            let fallo_w = &worker_fallo;
            std::thread::scope(|scope| -> Result<(), String> {
                let (tx, rx) = mpsc::sync_channel::<(usize, super::sitfa::Pagina, Vec<u8>, Vec<u8>, String)>(1);
                let (tx_evt, rx_evt) = mpsc::channel::<usize>();
                let carpeta = &self.carpeta;
                let worker = scope.spawn(move || -> (Vec<Value>, Option<Vec<String>>, Result<(), String>) {
                    let mut verificador = Verificador::default();
                    let mut res_local = Vec::new();
                    let mut cols_p1 = None;
                    let mut err = Ok(());
                    for (numero, pagina_n, html_n, datos_n, archivo_n) in rx {
                        match procesar_pagina_descargada(
                            carpeta,
                            &mut verificador,
                            &mut res_local,
                            consulta,
                            &pagina_n,
                            numero,
                            total,
                            archivo_n,
                            &datos_n,
                            &html_n,
                            listado,
                            caso,
                            manifiesto,
                        ) {
                            Ok(cols) => {
                                if numero == 1 {
                                    cols_p1 = cols;
                                }
                                let _ = tx_evt.send(numero);
                            }
                            Err(e) => {
                                fallo_w.store(true, Ordering::Release);
                                err = Err(e);
                                break;
                            }
                        }
                    }
                    (res_local, cols_p1, err)
                });

                let mut datos_p1 = datos_primera;
                let main_res = (|| -> Result<(), String> {
                    let mut actuales = pares.clone();
                    let mut pagina = primera.clone();
                    let mut html_actual = html_primera.to_vec();
                    let mut latencia_red = latencia_primera;
                    for numero in 1..=total {
                        cancelado(self.cancelar, CANCELADO_PAGINAS)?;
                        if worker_fallo.load(Ordering::Acquire) {
                            break;
                        }
                        for n_ok in rx_evt.try_iter() {
                            emitir(Evento::Pagina {
                                consulta: etiqueta_consulta.into(),
                                pagina: n_ok,
                                total,
                            });
                        }
                        let datos = if numero == 1 {
                            match datos_p1.take() {
                                Some(d) => d,
                                None => {
                                    validar_descarga(&pagina.excel_url, &primera.plantilla)?;
                                    cuerpo(
                                        &self.pedir("download", json!({"url": pagina.excel_url, "timeout": ESPERA_SITFA_MS}))?,
                                        true,
                                    )?
                                }
                            }
                        } else {
                            let pausa_restante = self.opciones.pausa.saturating_sub(latencia_red);
                            pausa(pausa_restante, self.cancelar, CANCELADO_PAGINAS)?;
                            if worker_fallo.load(Ordering::Acquire) {
                                break;
                            }
                            actuales = siguiente_consulta(&actuales, consulta, &pagina, numero)?;
                            let t0 = Instant::now();
                            let resp = self.pedir(
                                "search_and_download",
                                json!({
                                    "pairs": actuales,
                                    "timeout": ESPERA_SITFA_MS,
                                    "index_dom": false,
                                    "template": primera.plantilla,
                                }),
                            )?;
                            latencia_red = t0.elapsed();
                            let html = cuerpo(&resp["html"], false)?;
                            html_actual = html.clone();
                            pagina = match validar_respuesta(&html, &actuales, consulta, Some((numero, &primera)))? {
                                Respuesta::ConRegistros(p) => p,
                                Respuesta::SinRegistros => {
                                    return Err("No se pudieron identificar todos los registros de la pagina.".into())
                                }
                            };
                            validar_descarga(&pagina.excel_url, &primera.plantilla)?;
                            extraer_excel_compuesto(&resp, &pagina, &primera.plantilla, self)?
                        };
                        let archivo = nombre(numero, total);
                        if tx.send((numero, pagina.clone(), html_actual.clone(), datos, archivo)).is_err() {
                            break;
                        }
                    }
                    drop(tx);
                    Ok(())
                })();

                let (res_local, cols_p1, res_worker) = worker.join().unwrap_or_else(|_| {
                    (Vec::new(), None, Err("Fallo inesperado al verificar una página descargada.".into()))
                });
                resultados = res_local;
                if cols_p1.is_some() {
                    columnas_xls_p1 = cols_p1;
                }
                for n_ok in rx_evt.try_iter() {
                    emitir(Evento::Pagina {
                        consulta: etiqueta_consulta.into(),
                        pagina: n_ok,
                        total,
                    });
                }
                res_worker?;
                main_res?;
                if resultados.len() != total
                    || resultados
                        .iter()
                        .any(|r| !self.carpeta.join(r["archivo"].as_str().unwrap_or("")).is_file())
                {
                    return Err("La cantidad de XLS no coincide con el total de paginas.".into());
                }
                Ok(())
            })
        } else {
            (|| -> Result<(), String> {
                let mut verificador = Verificador::default();
                let mut actuales = pares.clone();
                let mut pagina = primera.clone();
                let mut html_actual = html_primera.to_vec();
                let mut datos_p1 = datos_primera;
                for numero in 1..=total {
                    cancelado(self.cancelar, CANCELADO_PAGINAS)?;
                    if numero > 1 {
                        pausa(self.opciones.pausa, self.cancelar, CANCELADO_PAGINAS)?;
                        actuales = siguiente_consulta(&actuales, consulta, &pagina, numero)?;
                        let html = cuerpo(
                            &self.pedir("search", json!({"pairs": actuales, "timeout": ESPERA_SITFA_MS}))?,
                            false,
                        )?;
                        html_actual = html.clone();
                        pagina = match validar_respuesta(&html, &actuales, consulta, Some((numero, &primera)))? {
                            Respuesta::ConRegistros(p) => p,
                            Respuesta::SinRegistros => {
                                return Err("No se pudieron identificar todos los registros de la pagina.".into())
                            }
                        };
                    }
                    let datos = if numero == 1 {
                        match datos_p1.take() {
                            Some(d) => d,
                            None => {
                                validar_descarga(&pagina.excel_url, &primera.plantilla)?;
                                cuerpo(
                                    &self.pedir("download", json!({"url": pagina.excel_url, "timeout": ESPERA_SITFA_MS}))?,
                                    true,
                                )?
                            }
                        }
                    } else {
                        validar_descarga(&pagina.excel_url, &primera.plantilla)?;
                        cuerpo(
                            &self.pedir("download", json!({"url": pagina.excel_url, "timeout": ESPERA_SITFA_MS}))?,
                            true,
                        )?
                    };
                    let archivo = nombre(numero, total);
                    if let Some(cols) = procesar_pagina_descargada(
                        &self.carpeta,
                        &mut verificador,
                        &mut resultados,
                        consulta,
                        &pagina,
                        numero,
                        total,
                        archivo,
                        &datos,
                        &html_actual,
                        listado,
                        caso,
                        manifiesto,
                    )? {
                        columnas_xls_p1 = Some(cols);
                    }
                    emitir(Evento::Pagina {
                        consulta: etiqueta_consulta.into(),
                        pagina: numero,
                        total,
                    });
                }
                if resultados.len() != total
                    || resultados
                        .iter()
                        .any(|r| !self.carpeta.join(r["archivo"].as_str().unwrap_or("")).is_file())
                {
                    return Err("La cantidad de XLS no coincide con el total de paginas.".into());
                }
                Ok(())
            })()
        };
        match ejecucion {
            Ok(()) => {
                self.manifiesto(manifiesto, caso, &resultados, total, "VALIDADA")?;
                Ok((resultados, columnas_xls_p1))
            }
            Err(e) => {
                let _ = self.manifiesto(manifiesto, caso, &resultados, total, "INCOMPLETA");
                Err(e)
            }
        }
    }

    /// `BatchRunner.capture_empty`: pantallazo de SITFA con la selección visible, guardado como PDF.
    fn evidencia(
        &self,
        perfil: &Perfil,
        consulta: &Consulta,
        pares: &Pares,
        fechas: Option<(NaiveDate, NaiveDate)>,
        nombre: &str,
        emitir: &mut dyn FnMut(Evento),
    ) -> Result<Value, String> {
        emitir(Evento::Estado("Consulta sin registros: capturando pantallazo…".into()));
        let integral = self.edicion == Edicion::Integral;
        let contexto = if integral {
            contexto_integral(perfil, self.catalogo, fechas)
        } else {
            contexto(perfil, self.catalogo, fechas)
        };
        let rotulo = caso_y_rotulo(perfil, self.catalogo).1;
        let resultado = (|| -> Result<Value, String> {
            let vista = self.pedir("evidence_open", json!({"pairs": pares, "label": rotulo, "context": contexto}))?;
            let html = cuerpo(&vista, false)?;
            let digest = huella(&html);
            if vista["digest"].as_str() != Some(digest.as_str()) {
                return Err("La pantalla de evidencia no coincide con la respuesta recibida.".into());
            }
            let textos = fechas.map(|(a, b)| (fecha_sitfa(a), fecha_sitfa(b)));
            validar_pantalla_evidencia(
                &html,
                pares,
                consulta,
                &contexto,
                textos.as_ref().map(|(a, b)| (a.as_str(), b.as_str())),
            )
            .map_err(|e| match (integral, e.ends_with("Recarga la extensión Lite.")) {
                (true, true) => "La pantalla no identifica el contexto de la consulta solicitada. Recarga la extensión de esta entrega.".into(),
                _ => e,
            })?;
            cancelado(self.cancelar, CANCELADO)?;
            let captura = self.pedir("evidence_capture", json!({"digest": digest}))?;
            let pdf = png_a_pdf(&captura, &digest, &rotulo)?;
            let suma = huella(&pdf);
            let ruta = self.carpeta.join(nombre);
            guardar_exclusivo(&ruta, &pdf)?;
            if fs::read(&ruta).map(|d| huella(&d)).ok().as_deref() != Some(suma.as_str()) {
                let _ = fs::remove_file(&ruta);
                return Err("No se pudo verificar la escritura del PDF.".into());
            }
            Ok(json!({
                "archivo": nombre, "registros": 0, "sha256": suma,
                "tipo": if integral { "PDF_CAPTURA_SITFA" } else { "PDF_PANTALLAZO_SIN_REGISTROS" },
                "contexto_consulta": contexto,
            }))
        })();
        let _ = self.conexion.llamar("evidence_close", json!({}), PLAZO_CIERRE);
        resultado
    }
}

fn contar_extension(carpeta: &Path, extension: &str) -> usize {
    fs::read_dir(carpeta)
        .map(|d| {
            d.filter_map(Result::ok)
                .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some(extension))
                .count()
        })
        .unwrap_or(0)
}

/// Consultas de un lote incompleto que faltan por descargar (`preferencias.pending_plan`).
#[derive(Debug, PartialEq, Eq)]
pub struct Pendientes {
    pub pantalla: Pantalla,
    pub selecciones: Vec<Seleccion>,
    pub fechas: Option<(NaiveDate, NaiveDate)>,
}

pub(super) fn sin_fecha_descarga(v: &Value) -> Value {
    let mut v = v.clone();
    if let Some(o) = v.as_object_mut() {
        o.remove("fecha_descarga");
    }
    v
}

/// Lee `resumen.json` de un lote y devuelve las consultas del plan que no figuran como completadas. Los filtros
/// privados preparados en SITFA no se guardan: un lote que los usó no se recupera. Las pendientes se descargan desde
/// la página 1 en un lote nuevo.
pub fn pendientes(carpeta: &Path) -> Result<Pendientes, String> {
    let texto = fs::read_to_string(carpeta.join("resumen.json"))
        .map_err(|_| "La carpeta elegida no tiene el resumen.json de un lote.".to_string())?;
    let resumen: Value = serde_json::from_str(&texto).map_err(|_| "El resumen.json del lote no se pudo leer.".to_string())?;
    let plan = resumen["plan"].as_array().filter(|p| !p.is_empty()).ok_or("El lote antiguo no tiene plan estructurado.")?;
    if resumen["otros_filtros"].as_str().is_some_and(|f| f.starts_with("formulario")) {
        return Err("Prepara otra vez los filtros privados; no se guardan para recuperación.".into());
    }
    let hechas: Vec<Value> = resumen["consultas"]
        .as_array()
        .map(|c| c.iter().map(|q| sin_fecha_descarga(&q["seleccion"])).collect())
        .unwrap_or_default();
    let faltan: Vec<&Value> = plan.iter().filter(|s| !hechas.contains(&sin_fecha_descarga(s))).collect();
    let primera = faltan.first().ok_or("No hay consultas pendientes.")?;
    let pantalla: Pantalla = serde_json::from_value(primera["screen"].clone()).map_err(|_| "El plan del lote no indica una pantalla conocida.")?;
    let fecha = |v: &Value| v.as_str().and_then(|t| NaiveDate::parse_from_str(t, "%d/%m/%Y").ok());
    let filtro = &resumen["filtro_fecha"];
    let fechas = fecha(&filtro["start"]).or_else(|| fecha(&filtro["desde"]))
        .zip(fecha(&filtro["end"]).or_else(|| fecha(&filtro["hasta"])));
    let texto = |v: &Value| v.as_str().unwrap_or("").to_owned();
    let mut selecciones = Vec::new();
    for s in faltan {
        if serde_json::from_value::<Pantalla>(s["screen"].clone()).ok() != Some(pantalla) {
            return Err("El plan pendiente mezcla pantallas distintas.".into());
        }
        selecciones.push(Seleccion {
            tribunal: texto(&s["tribunal"]),
            pestana: texto(&s["tab"]),
            modalidad: texto(&s["modality"]),
            reporte: texto(&s["report"]),
            estado: texto(&s["state"]),
            mes: texto(&s["month"]),
            anio: texto(&s["year"]),
            desde: fechas.map(|f| f.0),
            hasta: fechas.map(|f| f.1),
        });
    }
    Ok(Pendientes { pantalla, selecciones, fechas })
}

/// Valida los campos de filtro de fechas preparados por la extensión en el formulario SITFA.
/// Distingue bandera (`FLG_Consulta`), casilla (`CHK_Consulta`), inicio (`FEC_Inicio`) y fin (`FEC_Fin`)
/// para detener la búsqueda antes de enviarla a SITFA señalando el campo incoherente.
pub(super) fn validar_fechas_formulario(
    pares: &Pares,
    fechas: Option<(NaiveDate, NaiveDate)>,
) -> Result<(), String> {
    if let Some((a, b)) = fechas {
        if a > b {
            return Err("El rango de fechas está invertido (fecha inicial posterior a la final).".into());
        }
    }
    let campo = |k: &str| pares.iter().find(|(c, _)| c == k).map(|(_, v)| v.as_str());
    let bandera_esperada = if fechas.is_some() { "1" } else { "0" };
    if campo("FLG_Consulta") != Some(bandera_esperada) {
        return Err(if fechas.is_some() {
            "La bandera del filtro de fechas (FLG_Consulta) no quedó activa para el rango elegido.".into()
        } else {
            "La bandera del filtro de fechas (FLG_Consulta) quedó activa para «todas las fechas».".into()
        });
    }
    match fechas {
        Some((a, b)) => {
            if campo("CHK_Consulta") != Some("on") {
                return Err(
                    "La casilla del filtro de fechas (CHK_Consulta) no quedó activa para el rango elegido.".into(),
                );
            }
            let inicio = fecha_sitfa(a);
            if campo("FEC_Inicio") != Some(inicio.as_str()) {
                return Err(format!(
                    "La fecha inicial preparada (FEC_Inicio) no coincide con el rango elegido ({inicio})."
                ));
            }
            let fin = fecha_sitfa(b);
            if campo("FEC_Fin") != Some(fin.as_str()) {
                return Err(format!(
                    "La fecha final preparada (FEC_Fin) no coincide con el rango elegido ({fin})."
                ));
            }
        }
        None => {
            if campo("CHK_Consulta") == Some("on") {
                return Err(
                    "La casilla del filtro de fechas (CHK_Consulta) quedó activa para «todas las fechas».".into(),
                );
            }
        }
    }
    Ok(())
}

/// Selecciones del plan que se registran en el resumen y en las verificaciones.
pub(super) fn selecciones_plan(plan: &PlanLote, catalogo: &Catalogo, edicion: Edicion) -> Vec<Value> {
    let integral = edicion == Edicion::Integral;
    plan
        .consultas
        .iter()
        .map(|p| {
            let mut s = seleccion_edicion(p, edicion);
            s["tribunal_nombre"] = json!(etiqueta(&catalogo.tribunales, &p.tribunal));
            if integral {
                let modalidad = match &p.alcance {
                    Alcance::Seguimiento { modalidad, .. } => modalidad.as_str(),
                    _ => "",
                };
                s["modalidad_nombre"] = json!(MODALIDADES_LITE.iter().find(|(c, _)| *c == modalidad).map_or("", |(_, n)| n));
                s["fecha_descarga"] = json!(ahora());
            } else {
                s["etiquetas"] = json!(partes_consulta(p, catalogo, plan.fechas));
            }
            s
        })
        .collect()
}

/// Ejecuta el plan en una carpeta nueva dentro de `destino`. Devuelve las consultas completadas.
#[allow(clippy::too_many_arguments)]
pub fn ejecutar(
    conexion: &dyn Conexion,
    catalogo: &Catalogo,
    edicion: Edicion,
    plan: &PlanLote,
    destino: &Path,
    opciones: &Opciones,
    cancelar: &AtomicBool,
    emitir: &mut dyn FnMut(Evento),
) -> Result<Vec<Value>, String> {
    let integral = edicion == Edicion::Integral;
    if !integral && plan.consultas.iter().any(|p| p.pantalla == Pantalla::Carga) {
        return Err("Carga está disponible en Integral.".into());
    }
    if plan.consultas.is_empty() {
        return Err("El lote no tiene consultas.".into());
    }
    if plan.fechas.is_some_and(|(a, b)| a > b) {
        return Err("El rango de fechas está invertido (fecha inicial posterior a la final).".into());
    }
    if integral && !opciones.evidencia && !opciones.conservar_filtros && plan.consultas.len() > 1 {
        let solicitud = Solicitud { catalogo, edicion, plan, destino, opciones, cancelar };
        if let Some(resultado) = conexion.ejecutar_distribuido(&solicitud, emitir) {
            return resultado;
        }
    }
    let consultas: Vec<Consulta> = plan.consultas.iter().map(Consulta::desde).collect::<Result<_, _>>()?;
    let carpeta = carpeta_nueva(destino)?;
    // Lite anuncia la carpeta antes del primer resumen; Integral, después (E3B-14).
    if !integral {
        emitir(Evento::Carpeta(carpeta.clone()));
        emitir(Evento::Plan(plan.consultas.len()));
    }
    let lote = Lote { conexion, edicion, catalogo, opciones, cancelar, carpeta: carpeta.clone() };
    let limite = limite_nombre(&carpeta);
    let bases: Vec<String> = plan
        .consultas
        .iter()
        .map(|p| if integral { base_integral(p, catalogo) } else { base_archivo(p, catalogo, plan.fechas, limite) })
        .collect();
    let mut repetidas: HashMap<&str, usize> = HashMap::new();
    for b in &bases {
        *repetidas.entry(b.as_str()).or_default() += 1;
    }
    let usadas: std::cell::RefCell<HashMap<String, usize>> = Default::default();
    let unico = |base: &str, extension: &str| -> String {
        let mut nombre = base.to_string();
        if repetidas.get(base).copied().unwrap_or(0) > 1 {
            let mut usadas = usadas.borrow_mut();
            let n = usadas.entry(base.to_string()).or_default();
            *n += 1;
            nombre.push_str(&format!(" ({n})"));
        }
        nombre + extension
    };
    // Integral: ordinal « N» por base en todo el lote, contado por cada XLS y PDF.
    let ordinal = |base: &str, varias: bool| -> String {
        let mut usadas = usadas.borrow_mut();
        let n = usadas.entry(base.to_string()).or_default();
        *n += 1;
        if varias || repetidas.get(base).copied().unwrap_or(0) > 1 {
            format!("{base} {n}")
        } else {
            base.to_string()
        }
    };
    let planificadas = selecciones_plan(plan, catalogo, edicion);
    let mut completadas: Vec<Value> = Vec::new();
    let filtro_fecha = plan.fechas.map(|(a, b)| json!({"start": fecha_sitfa(a), "end": fecha_sitfa(b)}));
    let resumen = |estado: &str, completadas: &[Value]| -> Result<(), String> {
        if integral {
            return escribir_json(
                &carpeta,
                "resumen.json",
                &json!({
                    "version_contrato": 1, "fecha_hora": ahora(), "estado": estado, "plan": planificadas,
                    "consultas_esperadas": plan.consultas.len(), "consultas_completadas": completadas.len(),
                    "consultas": completadas,
                    "filtro_fecha": plan.fechas.map(|(a, b)| json!({"desde": fecha_sitfa(a), "hasta": fecha_sitfa(b)})),
                    "otros_filtros": if opciones.conservar_filtros { "formulario" } else { "todos los registros" },
                }),
            );
        }
        escribir_json(
            &carpeta,
            "resumen.json",
            &json!({
                "version": "lite-1", "fecha_hora": ahora(), "estado": estado,
                "consultas_esperadas": plan.consultas.len(), "consultas_completadas": completadas.len(),
                "filtro_fecha": filtro_fecha,
                "otros_filtros": if opciones.conservar_filtros { "formulario SITFA" } else { "todos los registros" },
                "consultas": completadas, "plan": planificadas,
            }),
        )
    };
    resumen("EN_CURSO", &completadas)?;
    if integral {
        emitir(Evento::Carpeta(carpeta.clone()));
        emitir(Evento::Plan(plan.consultas.len()));
    }
    let mut bloqueado = false;
    let mut diagnostico_consultas: Vec<Value> = Vec::new();
    let ejecucion = (|| -> Result<(), String> {
        lote.pedir("lock", json!({}))?;
        bloqueado = true;
        for (i, (perfil, consulta)) in plan.consultas.iter().zip(&consultas).enumerate() {
            let indice = i + 1;
            cancelado(cancelar, CANCELADO)?;
            if indice > 1 && !opciones.pausa.is_zero() {
                pausa(opciones.pausa, cancelar, CANCELADO)?;
            }
            let base = &bases[i];
            let caso = caso_y_rotulo(perfil, catalogo);
            // En Integral la consulta se identifica por la etiqueta del perfil; en Lite, por el nombre del archivo.
            let rotulo = if integral { caso.1.clone() } else { base.clone() };
            emitir(Evento::Consulta { indice, total: plan.consultas.len(), etiqueta: rotulo.clone() });
            let fechas = if integral { plan.fechas } else { plan.fechas.filter(|_| !calendario(perfil.pantalla)) };
            let preparado = lote.pedir(
                "prepare",
                json!({
                    "selection": seleccion_edicion(perfil, edicion),
                    "dates": fechas.map(|(a, b)| json!({"start": fecha_sitfa(a), "end": fecha_sitfa(b)})),
                    "keep_filters": opciones.conservar_filtros && (integral || !calendario(perfil.pantalla)),
                }),
            )?;
            let pares: Pares = preparado["pairs"]
                .as_array()
                .filter(|a| a.len() <= 100)
                .and_then(|a| {
                    a.iter()
                        .map(|par| match par.as_array().map(Vec::as_slice) {
                            Some([Value::String(k), Value::String(v)]) => Some((k.clone(), v.clone())),
                            _ => None,
                        })
                        .collect()
                })
                .ok_or("El formulario no entregó parámetros válidos.")?;
            consulta.validar_consulta(&pares)?;
            // `descargador/lotes.py`: Carga no usa FLG_Consulta ni FEC_Inicio/FEC_Fin; su período va en el objetivo.
            if !calendario(perfil.pantalla) && perfil.pantalla != Pantalla::Carga {
                validar_fechas_formulario(&pares, fechas)?;
            }
            let (html, respuesta_compuesta, latencia_p1) = if lote.conexion.soporta_search_and_download() {
                let mut payload = json!({"pairs": pares, "timeout": ESPERA_SITFA_MS, "index_dom": false});
                if let Some(tpl) = consulta.plantilla {
                    payload["template"] = json!(tpl);
                }
                let t0 = Instant::now();
                let resp = lote.pedir("search_and_download", payload)?;
                let latencia = t0.elapsed();
                let html = cuerpo(&resp["html"], false)?;
                (html, Some(resp), latencia)
            } else {
                let html = cuerpo(&lote.pedir("search", json!({"pairs": pares, "timeout": ESPERA_SITFA_MS}))?, false)?;
                (html, None, Duration::ZERO)
            };
            let manifiesto = format!("verificacion {indice:03}.json");
            let mut evidencia = None;
            let resultado = match validar_respuesta(&html, &pares, consulta, None)? {
                Respuesta::SinRegistros => {
                    cancelado(cancelar, CANCELADO)?;
                    if opciones.evidencia {
                        let nombre = if integral {
                            ordinal(base, false) + ".pdf"
                        } else {
                            unico(base, " - SIN REGISTROS.pdf")
                        };
                        evidencia = Some(lote.evidencia(perfil, consulta, &pares, fechas, &nombre, emitir)?);
                    }
                    lote.manifiesto(&manifiesto, &caso, &[], 0, "SIN_RESULTADOS")?;
                    match &evidencia {
                        Some(e) => emitir(Evento::Evidencia { archivo: e["archivo"].as_str().unwrap_or("").into() }),
                        None => emitir(Evento::Vacia { consulta: rotulo.clone() }),
                    }
                    Vec::new()
                }
                Respuesta::ConRegistros(primera) => {
                    emitir(Evento::Total(primera.total));
                    let datos_primera = match respuesta_compuesta {
                        Some(ref resp) if resp["excel"].is_object() => {
                            Some(extraer_excel_compuesto(resp, &primera, &primera.plantilla, &lote)?)
                        }
                        _ => None,
                    };
                    let raiz = std::cell::OnceCell::new();
                    let nombre = |numero: usize, total: usize| {
                        if integral {
                            return ordinal(base, total > 1) + ".xls";
                        }
                        let raiz = raiz.get_or_init(|| unico(base, ""));
                        format!("{raiz}{}.xls", if total > 1 { format!(" - p{numero} de {total}") } else { String::new() })
                    };
                    let listado = match &perfil.alcance {
                        Alcance::Seguimiento { pestana, .. } => Some(super::vinculos::Seleccion {
                            tribunal: perfil.tribunal.clone(),
                            screen: "seguimiento".into(),
                            tab: pestana.clone(),
                        }),
                        _ => None,
                    };
                    let (filas, cols_xls_p1) = lote.paginas(
                        &pares,
                        primera,
                        consulta,
                        &caso,
                        &manifiesto,
                        &nombre,
                        emitir,
                        &rotulo,
                        &html,
                        datos_primera,
                        latencia_p1,
                        listado.as_ref(),
                    )?;
                    if let (Alcance::Seguimiento { pestana, modalidad, .. }, Some(columnas_xls)) = (&perfil.alcance, cols_xls_p1) {
                        let columnas_html = super::sitfa::cabeceras_tabla_seguimiento(&html, consulta.tabla);
                        let (solo_en_xls, solo_en_html) = super::sitfa::diferencia_columnas(&columnas_html, &columnas_xls);
                        let modalidad_nombre = MODALIDADES_LITE
                            .iter()
                            .find(|(c, _)| c == modalidad)
                            .map(|(_, n)| (*n).to_string())
                            .unwrap_or_else(|| etiqueta(&catalogo.modalidades, modalidad).to_string());
                        diagnostico_consultas.push(json!({
                            "pestana": pestana,
                            "modalidad": modalidad_nombre,
                            "columnas_html": columnas_html,
                            "columnas_xls": columnas_xls,
                            "solo_en_xls": solo_en_xls,
                            "solo_en_html": solo_en_html,
                        }));
                    }
                    filas
                }
            };
            let archivos: Vec<Value> = if resultado.is_empty() {
                evidencia.iter().map(|e| e["archivo"].clone()).collect()
            } else {
                resultado.iter().map(|r| r["archivo"].clone()).collect()
            };
            let mut hecha = json!({
                "consulta": rotulo,
                "estado": if resultado.is_empty() { "SIN_RESULTADOS" } else { "VALIDADA" },
                "paginas": resultado.len(),
                "registros": resultado.iter().map(|r| r["registros"].as_u64().unwrap_or(0)).sum::<u64>(),
                "archivos": archivos, "verificacion": manifiesto, "seleccion": planificadas[i],
            });
            if integral {
                if let Some(o) = hecha.as_object_mut() {
                    o.remove("archivos");
                }
                if let Some(e) = &evidencia {
                    hecha["evidencia"] = e.clone();
                }
            }
            let ruta = carpeta.join(&manifiesto);
            let mut info: Value = fs::read(&ruta)
                .ok()
                .and_then(|d| serde_json::from_slice(&d).ok())
                .ok_or("No se pudo releer el manifiesto de la consulta.")?;
            info["seleccion"] = planificadas[i].clone();
            info["filtro_fecha"] = json!(fechas.map(|(a, b)| json!({"start": fecha_sitfa(a), "end": fecha_sitfa(b)})));
            if let Some(e) = &evidencia {
                info["evidencia"] = e.clone();
            } else if resultado.is_empty() {
                info["evidencia_omitida"] = json!(true);
                info["comprobante_sin_registros"] = json!({
                    "tipo": "JSON_SIN_REGISTROS",
                    "fecha_hora": ahora(),
                    "sha256_html": huella(&html),
                    "contexto_consulta": if integral {
                        contexto_integral(perfil, catalogo, fechas)
                    } else {
                        contexto(perfil, catalogo, fechas)
                    },
                });
                hecha["evidencia_omitida"] = json!(true);
            }
            escribir_json(&carpeta, &manifiesto, &info)?;
            completadas.push(hecha);
            resumen("EN_CURSO", &completadas)?;
            emitir(Evento::ConsultaLista {
                consulta: rotulo.clone(),
                estado: completadas.last().map(|c| c["estado"].as_str().unwrap_or("").to_string()).unwrap_or_default(),
            });
        }
        if !diagnostico_consultas.is_empty() {
            escribir_json(
                &carpeta,
                "diagnostico_columnas_html_xls.json",
                &json!({
                    "version": 1,
                    "pantalla": "seguimiento",
                    "consultas": diagnostico_consultas,
                }),
            )?;
        }
        Ok(())
    })();
    let ejecucion = ejecucion.and_then(|()| {
        resumen("VALIDADA", &completadas)?;
        emitir(Evento::Fin(Fin {
            carpeta: carpeta.clone(),
            consultas: completadas.len(),
            archivos: contar_extension(&carpeta, "xls"),
            pdfs: contar_extension(&carpeta, "pdf"),
            vacias: completadas.iter().filter(|c| c["estado"] == "SIN_RESULTADOS").count(),
            registros: completadas.iter().map(|c| c["registros"].as_u64().unwrap_or(0) as usize).sum(),
        }));
        Ok(())
    });
    if ejecucion.is_err() {
        let _ = resumen("INCOMPLETA", &completadas);
    }
    if integral {
        // Como el `finally` de Integral: el alcance queda también en la verificación de la consulta interrumpida.
        for (i, s) in planificadas.iter().enumerate() {
            let nombre = format!("verificacion {:03}.json", i + 1);
            let Ok(datos) = fs::read(carpeta.join(&nombre)) else { continue };
            match serde_json::from_slice::<Value>(&datos) {
                Ok(mut info) if info.get("seleccion").is_none() => {
                    info["seleccion"] = s.clone();
                    if escribir_json(&carpeta, &nombre, &info).is_err() {
                        emitir(Evento::Aviso("E_RESUMEN: no se pudo completar el alcance de una verificación.".into()));
                    }
                }
                Ok(_) => {}
                Err(_) => emitir(Evento::Aviso("E_RESUMEN: no se pudo completar el alcance de una verificación.".into())),
            }
        }
    }
    if bloqueado {
        match conexion.llamar("unlock", json!({}), PLAZO_CIERRE) {
            Ok(v) if v["restored"] == json!(false) => emitir(Evento::Aviso(if integral {
                "No se restauraron todos los filtros de SITFA. Revisa el formulario antes de volver a consultar; los archivos verificados se conservan.".into()
            } else {
                "No se restauraron todos los filtros de SITFA. Revisa el formulario.".into()
            })),
            Ok(_) => {}
            Err(_) => emitir(Evento::Aviso(if integral {
                "No se pudo confirmar la restauración de los filtros de SITFA. Revisa la pestaña antes de volver a consultar; los archivos verificados se conservan.".into()
            } else {
                "No se pudo confirmar la restauración de los filtros de SITFA.".into()
            })),
        }
    }
    ejecucion.map(|()| completadas)
}

#[cfg(test)]
mod pruebas_lote_python {
    use super::*;
    use crate::descargador::{SeleccionLote, Catalogo};
    use std::cell::RefCell;

    /// Reproduce las respuestas que el `BatchRunner` de Flow Python recibió del puente ficticio, en el mismo orden.
    struct Guion {
        pasos: Vec<Value>,
        pedidos: RefCell<Vec<Value>>,
    }

    impl Conexion for Guion {
        fn llamar(&self, comando: &str, payload: Value, _plazo: Duration) -> Result<Value, String> {
            let n = self.pedidos.borrow().len();
            self.pedidos.borrow_mut().push(json!({"comando": comando, "payload": payload}));
            let paso = self.pasos.get(n).ok_or_else(|| format!("orden {n} ({comando}) fuera del guion de Python"))?;
            if paso["comando"] != comando {
                return Err(format!("orden {n}: Rust pide {comando}, Python pidió {}", paso["comando"]));
            }
            paso.get("respuesta").cloned().ok_or_else(|| paso["error"].as_str().unwrap_or("error").to_string())
        }
    }

    fn sin_vacio(v: &Value) -> Value {
        if v.is_null() || v.as_object().is_some_and(serde_json::Map::is_empty) { Value::Null } else { v.clone() }
    }

    fn sin_horas(v: &Value) -> Value {
        match v {
            Value::Object(m) => Value::Object(
                m.iter()
                    .map(|(k, x)| {
                        let x = if k == "fecha_hora" || k == "fecha_descarga" { json!("<HORA>") } else { sin_horas(x) };
                        (k.clone(), x)
                    })
                    .collect(),
            ),
            Value::Array(a) => Value::Array(a.iter().map(sin_horas).collect()),
            otro => otro.clone(),
        }
    }

    /// E-Q5: el lote de listados Integral de Rust da las mismas órdenes, el mismo `resumen.json`, los mismos
    /// manifiestos (con `vinculos_ingreso` de cada página de Seguimiento) y los mismos XLS que `lotes.BatchRunner`.
    #[test]
    fn lote_de_listados_igual_a_python() {
        let datos: Value = serde_json::from_str(include_str!("../../tests/datos/lote.json")).unwrap();
        let catalogo: Catalogo = serde_json::from_value(datos["catalogo"].clone()).unwrap();
        let escenarios = datos["escenarios"].as_array().unwrap();
        let (mut iguales, mut fallos) = (0usize, Vec::new());
        for e in escenarios {
            let nombre = e["escenario"].as_str().unwrap();
            let lista = |k: &str| -> Vec<String> { serde_json::from_value(e["lote"][k].clone()).unwrap() };
            let seleccion = SeleccionLote {
                tribunales: lista("tribunales"),
                modalidades: lista("modalidades"),
                pestanas: lista("pestanas"),
                informes: vec!["1".into()],
                // Cumplimiento sin filtro de medidas (`report` nulo en Python): «Todas las medidas».
                medidas: vec![crate::descargador::TODAS_LAS_MEDIDAS.into()],
                ..Default::default()
            };
            let plan = catalogo.planificar(Edicion::Integral, &seleccion).unwrap();
            let guion = Guion { pasos: e["guion"].as_array().unwrap().clone(), pedidos: RefCell::default() };
            let destino = std::env::temp_dir().join(format!("flow-eq5-lote-{}-{nombre}", std::process::id()));
            let _ = fs::remove_dir_all(&destino);
            let mut carpeta = PathBuf::new();
            let resultado = ejecutar(&guion, &catalogo, Edicion::Integral, &plan, &destino, &Opciones::default(),
                &AtomicBool::new(false), &mut |ev| if let Evento::Carpeta(c) = ev { carpeta = c });
            let mut diferencias = Vec::new();
            if let Err(err) = &resultado {
                diferencias.push(format!("ejecutar: {err}"));
            }
            let pedidos = guion.pedidos.borrow();
            let esperados = e["guion"].as_array().unwrap();
            if pedidos.len() != esperados.len() {
                diferencias.push(format!("órdenes: Rust {} / Python {}", pedidos.len(), esperados.len()));
            }
            for (k, (p, x)) in pedidos.iter().zip(esperados).enumerate() {
                if p["comando"] != x["comando"] || sin_vacio(&p["payload"]) != sin_vacio(&x["payload"]) {
                    diferencias.push(format!("orden {k}: Rust {p} / Python {}", json!({"comando": x["comando"], "payload": x["payload"]})));
                }
            }
            for (archivo, esperado) in e["jsons"].as_object().unwrap() {
                match fs::read(carpeta.join(archivo)) {
                    Ok(b) => {
                        let obtenido = sin_horas(&serde_json::from_slice::<Value>(&b).unwrap());
                        if &obtenido != esperado {
                            diferencias.push(format!("{archivo}:\n    Rust   {obtenido}\n    Python {esperado}"));
                        }
                    }
                    Err(_) => diferencias.push(format!("{archivo}: falta en Rust")),
                }
            }
            for (archivo, huella) in e["huellas"].as_object().unwrap() {
                let obtenida = fs::read(carpeta.join(archivo)).map(|b| huella_archivo(&b)).unwrap_or_default();
                if obtenida != huella.as_str().unwrap() {
                    diferencias.push(format!("{archivo}: huella distinta o ausente"));
                }
            }
            let _ = fs::remove_dir_all(&destino);
            if diferencias.is_empty() {
                iguales += 1;
            } else {
                fallos.push(format!("{nombre}:\n  {}", diferencias.join("\n  ")));
            }
        }
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("LOTE: {iguales}/{}, fallos {}", escenarios.len(), fallos.len());
        assert_eq!(escenarios.len(), 2, "el fixture cambió de tamaño: regenerar y revisar");
        assert!(fallos.is_empty(), "{} escenarios difieren de Flow Python", fallos.len());
    }

    fn huella_archivo(b: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        format!("{:x}", Sha256::digest(b))
    }

    fn pares_con_fechas(
        base: &Value,
        flg: Option<&str>,
        chk: Option<&str>,
        inicio: Option<&str>,
        fin: Option<&str>,
    ) -> Value {
        let mut pares: Vec<Value> = base
            .as_array()
            .unwrap()
            .iter()
            .filter(|par| {
                !matches!(
                    par.as_array().and_then(|a| a.first()).and_then(Value::as_str),
                    Some("FLG_Consulta" | "CHK_Consulta" | "FEC_Inicio" | "FEC_Fin")
                )
            })
            .cloned()
            .collect();
        for (k, v) in [("FLG_Consulta", flg), ("CHK_Consulta", chk), ("FEC_Inicio", inicio), ("FEC_Fin", fin)] {
            if let Some(v) = v {
                pares.push(json!([k, v]));
            }
        }
        Value::Array(pares)
    }

    #[test]
    fn validar_fechas_formulario_distingue_bandera_casilla_inicio_fin_y_todas_las_fechas() {
        let d1 = NaiveDate::from_ymd_opt(2099, 3, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2099, 3, 31).unwrap();
        let par = |items: &[(&str, &str)]| -> Pares {
            items.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
        };

        // Rango válido
        let ok_rango = par(&[
            ("FLG_Consulta", "1"),
            ("CHK_Consulta", "on"),
            ("FEC_Inicio", "01/03/2099"),
            ("FEC_Fin", "31/03/2099"),
        ]);
        assert_eq!(validar_fechas_formulario(&ok_rango, Some((d1, d2))), Ok(()));

        // Bandera incorrecta
        let err_flg = validar_fechas_formulario(
            &par(&[("FLG_Consulta", "0"), ("CHK_Consulta", "on"), ("FEC_Inicio", "01/03/2099"), ("FEC_Fin", "31/03/2099")]),
            Some((d1, d2)),
        )
        .unwrap_err();
        assert!(err_flg.contains("FLG_Consulta"), "{err_flg}");

        // Casilla inactiva
        let err_chk = validar_fechas_formulario(
            &par(&[("FLG_Consulta", "1"), ("FEC_Inicio", "01/03/2099"), ("FEC_Fin", "31/03/2099")]),
            Some((d1, d2)),
        )
        .unwrap_err();
        assert!(err_chk.contains("CHK_Consulta"), "{err_chk}");

        // Fecha inicial distinta
        let err_ini = validar_fechas_formulario(
            &par(&[("FLG_Consulta", "1"), ("CHK_Consulta", "on"), ("FEC_Inicio", "02/03/2099"), ("FEC_Fin", "31/03/2099")]),
            Some((d1, d2)),
        )
        .unwrap_err();
        assert!(err_ini.contains("FEC_Inicio"), "{err_ini}");

        // Fecha final distinta
        let err_fin = validar_fechas_formulario(
            &par(&[("FLG_Consulta", "1"), ("CHK_Consulta", "on"), ("FEC_Inicio", "01/03/2099"), ("FEC_Fin", "30/03/2099")]),
            Some((d1, d2)),
        )
        .unwrap_err();
        assert!(err_fin.contains("FEC_Fin"), "{err_fin}");

        // «Todas las fechas» válido
        assert_eq!(validar_fechas_formulario(&par(&[("FLG_Consulta", "0")]), None), Ok(()));

        // «Todas las fechas» con bandera activa
        let err_todas_flg = validar_fechas_formulario(&par(&[("FLG_Consulta", "1")]), None).unwrap_err();
        assert!(err_todas_flg.contains("FLG_Consulta"), "{err_todas_flg}");

        // «Todas las fechas» con casilla activa
        let err_todas_chk =
            validar_fechas_formulario(&par(&[("FLG_Consulta", "0"), ("CHK_Consulta", "on")]), None).unwrap_err();
        assert!(err_todas_chk.contains("CHK_Consulta"), "{err_todas_chk}");

        // Rango invertido
        let err_inv = validar_fechas_formulario(&ok_rango, Some((d2, d1))).unwrap_err();
        assert!(err_inv.contains("invertido"), "{err_inv}");
    }

    #[test]
    fn lote_acepta_rango_valido_y_detiene_busqueda_ante_cada_discrepancia_de_fechas() {
        let datos: Value = serde_json::from_str(include_str!("../../tests/datos/lote.json")).unwrap();
        let catalogo: Catalogo = serde_json::from_value(datos["catalogo"].clone()).unwrap();
        let escenario = datos["escenarios"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["escenario"] == "sin_registros")
            .unwrap_or(&datos["escenarios"][0]);
        let lista = |k: &str| -> Vec<String> { serde_json::from_value(escenario["lote"][k].clone()).unwrap() };
        let pasos_base = escenario["guion"].as_array().unwrap();
        let prepare_base = pasos_base.iter().find(|p| p["comando"] == "prepare").unwrap();
        let search_base = pasos_base.iter().find(|p| p["comando"] == "search").unwrap();
        let d1 = NaiveDate::from_ymd_opt(2099, 3, 1).unwrap();
        let d2 = NaiveDate::from_ymd_opt(2099, 3, 31).unwrap();

        let hacer_plan = |fechas: Option<(NaiveDate, NaiveDate)>| -> PlanLote {
            let seleccion = SeleccionLote {
                tribunales: vec![lista("tribunales")[0].clone()],
                modalidades: vec![lista("modalidades")[0].clone()],
                pestanas: vec![lista("pestanas")[0].clone()],
                informes: vec!["1".into()],
                medidas: vec![crate::descargador::TODAS_LAS_MEDIDAS.into()],
                fechas,
                ..Default::default()
            };
            catalogo.planificar(Edicion::Integral, &seleccion).unwrap()
        };

        // 1) Rango correcto: ejecuta prepare y search
        let mut pasos_ok = Vec::new();
        for p in pasos_base {
            let mut copia = p.clone();
            if copia["comando"] == "prepare" {
                copia["respuesta"]["pairs"] = pares_con_fechas(
                    &prepare_base["respuesta"]["pairs"],
                    Some("1"),
                    Some("on"),
                    Some("01/03/2099"),
                    Some("31/03/2099"),
                );
            }
            pasos_ok.push(copia);
            if p["comando"] == "unlock" {
                break;
            }
        }
        // Si el escenario base tenía >1 consulta, construimos una corrida limpia de 1 consulta con search vacío o existente
        let pasos_uno = if pasos_base.iter().filter(|p| p["comando"] == "prepare").count() == 1 {
            pasos_ok
        } else {
            vec![
                json!({"comando": "lock", "respuesta": {"ok": true}}),
                json!({
                    "comando": "prepare",
                    "respuesta": {"pairs": pares_con_fechas(&prepare_base["respuesta"]["pairs"], Some("1"), Some("on"), Some("01/03/2099"), Some("31/03/2099"))}
                }),
                search_base.clone(),
                json!({"comando": "unlock", "respuesta": {"restored": true}}),
            ]
        };
        let guion_ok = Guion { pasos: pasos_uno, pedidos: RefCell::default() };
        let destino_ok = std::env::temp_dir().join(format!("flow-lote-fechas-ok-{}", std::process::id()));
        let _ = fs::remove_dir_all(&destino_ok);
        let res_ok = ejecutar(
            &guion_ok,
            &catalogo,
            Edicion::Integral,
            &hacer_plan(Some((d1, d2))),
            &destino_ok,
            &Opciones { evidencia: false, ..Default::default() },
            &AtomicBool::new(false),
            &mut |_| {},
        );
        // Puede terminar en search (y pedir download si tenía registros); lo crucial es que alcanzó `search` con las fechas válidas
        assert!(
            guion_ok.pedidos.borrow().iter().any(|p| p["comando"] == "search"),
            "el rango válido debió avanzar hasta search: res={res_ok:?}"
        );
        let _ = fs::remove_dir_all(&destino_ok);

        // 2) Cada discrepancia detiene ANTES de search y nombra el campo
        let casos_malos: [(&str, Option<(NaiveDate, NaiveDate)>, Option<&str>, Option<&str>, Option<&str>, Option<&str>, &str); 6] = [
            ("flg_malo", Some((d1, d2)), Some("0"), Some("on"), Some("01/03/2099"), Some("31/03/2099"), "FLG_Consulta"),
            ("chk_inactiva", Some((d1, d2)), Some("1"), None, Some("01/03/2099"), Some("31/03/2099"), "CHK_Consulta"),
            ("ini_distinta", Some((d1, d2)), Some("1"), Some("on"), Some("02/03/2099"), Some("31/03/2099"), "FEC_Inicio"),
            ("fin_distinta", Some((d1, d2)), Some("1"), Some("on"), Some("01/03/2099"), Some("30/03/2099"), "FEC_Fin"),
            ("todas_flg_activo", None, Some("1"), None, None, None, "FLG_Consulta"),
            ("todas_chk_activo", None, Some("0"), Some("on"), None, None, "CHK_Consulta"),
        ];
        for (etiqueta, fechas, flg, chk, ini, fin, campo_esperado) in casos_malos {
            let guion = Guion {
                pasos: vec![
                    json!({"comando": "lock", "respuesta": {"ok": true}}),
                    json!({
                        "comando": "prepare",
                        "respuesta": {"pairs": pares_con_fechas(&prepare_base["respuesta"]["pairs"], flg, chk, ini, fin)}
                    }),
                    json!({"comando": "unlock", "respuesta": {"restored": true}}),
                ],
                pedidos: RefCell::default(),
            };
            let destino = std::env::temp_dir().join(format!("flow-lote-fechas-{etiqueta}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&destino);
            let err = ejecutar(
                &guion,
                &catalogo,
                Edicion::Integral,
                &hacer_plan(fechas),
                &destino,
                &Opciones::default(),
                &AtomicBool::new(false),
                &mut |_| {},
            )
            .unwrap_err();
            let _ = fs::remove_dir_all(&destino);
            assert!(err.contains(campo_esperado), "{etiqueta}: esperaba {campo_esperado}, obtuvo {err}");
            assert!(
                guion.pedidos.borrow().iter().all(|p| p["comando"] != "search"),
                "{etiqueta}: no debía iniciar search"
            );
        }

        // 3) Rango invertido se rechaza antes de tocar el puente (0 comandos)
        let mut plan_inv = hacer_plan(Some((d1, d2)));
        plan_inv.fechas = Some((d2, d1));
        let guion_inv = Guion { pasos: vec![], pedidos: RefCell::default() };
        let destino_inv = std::env::temp_dir().join(format!("flow-lote-fechas-inv-{}", std::process::id()));
        let err_inv = ejecutar(
            &guion_inv,
            &catalogo,
            Edicion::Integral,
            &plan_inv,
            &destino_inv,
            &Opciones::default(),
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap_err();
        assert!(err_inv.contains("invertido"), "{err_inv}");
        assert!(guion_inv.pedidos.borrow().is_empty());
    }

    struct MockMultipagina {
        soporta_compuesto: bool,
        total_paginas: usize,
        vacia: bool,
        demora: Duration,
        incluir_rut_xls: bool,
        xls_corrupto_p1: bool,
        llamadas: RefCell<Vec<(String, Value)>>,
        tiempos_sad: RefCell<Vec<(Instant, Instant)>>,
        ultima_pagina: RefCell<usize>,
    }

    impl MockMultipagina {
        fn nuevo(soporta_compuesto: bool, total_paginas: usize) -> Self {
            Self {
                soporta_compuesto,
                total_paginas,
                vacia: false,
                demora: Duration::ZERO,
                incluir_rut_xls: false,
                xls_corrupto_p1: false,
                llamadas: RefCell::new(Vec::new()),
                tiempos_sad: RefCell::new(Vec::new()),
                ultima_pagina: RefCell::new(1),
            }
        }

        fn catalogo_seguimiento() -> Catalogo {
            serde_json::from_value(json!({
                "pantalla": "seguimiento",
                "tribunales": {"901": "Juzgado de Familia Ficticio"},
                "pestanas": ["Cumplimiento"],
                "modalidades": {"2": "Ambulatorio"},
                "informes": {"1": "Por vencer"},
                "medidas": {"0": "Todas"}
            }))
            .unwrap()
        }

        fn plan_cumplimiento(catalogo: &Catalogo) -> PlanLote {
            let seleccion = SeleccionLote {
                tribunales: vec!["901".into()],
                pestanas: vec!["Cumplimiento".into()],
                modalidades: vec!["2".into()],
                medidas: vec![crate::descargador::TODAS_LAS_MEDIDAS.into()],
                ..Default::default()
            };
            catalogo.planificar(Edicion::Integral, &seleccion).unwrap()
        }

        fn html_pagina(&self, pagina: usize) -> Vec<u8> {
            let url_xls = "https://familia.pjud.cl/sitfa/reportes/u1_amblistcump.xls";
            let (cuerpo, script) = if self.vacia {
                (
                    "<tr><td>Sin registros</td></tr>".to_string(),
                    String::new(),
                )
            } else {
                let filas = if self.incluir_rut_xls {
                    format!(
                        "<tr><td>C-{pagina}1-2099</td><td>Persona Secreta Ficticia {pagina}</td><td>Programa Ficticio Norte</td></tr>\
                        <tr><td>C-{pagina}2-2099</td><td>contacto.ficticio@example.test</td><td>Programa Ficticio Norte</td></tr>"
                    )
                } else {
                    format!(
                        "<tr><td>C-{pagina}1-2099</td><td>Persona Secreta Ficticia {pagina}</td><td>Programa Ficticio Norte</td></tr>"
                    )
                };
                (
                    filas,
                    format!(r#"function ShowExcel(){{window.open("{url_xls}");}}"#),
                )
            };
            format!(
                r#"<html><body><form name="InformesPpalForm" method="post" action="/SITFAWEB/InformesDAction.do">
                <select name="COD_Tribunal_sel"><option value="901" selected>901</option></select>
                <select name="TIP_Consulta"><option value="2" selected>2</option></select>
                <select name="TIP_Informe"><option value="0" selected>0</option></select>
                <input name="COD_Lengueta" value="">
                <input name="NUM_PaginaCumplimiento" value="{pagina}">
                <input name="NUM_TotalCumplimiento" value="{}">
                <table id="tablaCumplimiento"><tr><th>RIT</th><th>Nombre</th><th>Programa</th></tr>{cuerpo}</table>
                </form><script>{script}</script></body></html>"#,
                if self.vacia { 1 } else { self.total_paginas }
            )
            .into_bytes()
        }

        fn xls_pagina(&self, pagina: usize) -> Vec<u8> {
            if self.incluir_rut_xls {
                format!(
                    "<html><table><tr><th>RIT</th><th>RUT</th><th>Nombre</th><th>Programa</th></tr>\
                    <tr><td>C-{pagina}1-2099</td><td>11.111.111-1</td><td>Persona Secreta Ficticia {pagina}</td><td>Programa Ficticio Norte</td></tr>\
                    <tr><td>C-{pagina}2-2099</td><td>11.111.111-1</td><td>contacto.ficticio@example.test</td><td>Programa Ficticio Norte</td></tr></table></html>"
                )
                .into_bytes()
            } else {
                format!(
                    "<html><table><tr><th>RIT</th><th>Nombre</th><th>Programa</th></tr>\
                    <tr><td>C-{pagina}1-2099</td><td>Persona Secreta Ficticia {pagina}</td><td>Programa Ficticio Norte</td></tr></table></html>"
                )
                .into_bytes()
            }
        }

        fn pagina_de_pares(pairs: &Value) -> usize {
            pairs
                .as_array()
                .and_then(|a| {
                    a.iter()
                        .find(|p| p[0] == "NUM_PaginaCumplimiento")
                        .and_then(|p| p[1].as_str())
                        .and_then(|s| s.parse::<usize>().ok())
                })
                .unwrap_or(1)
        }
    }

    impl Conexion for MockMultipagina {
        fn soporta_search_and_download(&self) -> bool {
            self.soporta_compuesto
        }

        fn llamar(&self, comando: &str, payload: Value, _plazo: Duration) -> Result<Value, String> {
            self.llamadas.borrow_mut().push((comando.to_string(), payload.clone()));
            match comando {
                "lock" | "unlock" => Ok(json!({"ok": true, "restored": true})),
                "prepare" => Ok(json!({
                    "pairs": [
                        ["COD_Tribunal_sel", "901"],
                        ["COD_Lengueta", "tdCumplimiento"],
                        ["TIP_Consulta", "2"],
                        ["TIP_Informe", "0"],
                        ["NUM_PaginaCumplimiento", ""],
                        ["NUM_TotalCumplimiento", ""],
                        ["FLG_Consulta", "0"],
                        ["irAccion", "Buscar medida"]
                    ]
                })),
                "search" => {
                    let pag = Self::pagina_de_pares(&payload["pairs"]);
                    *self.ultima_pagina.borrow_mut() = pag;
                    Ok(json!({
                        "status": 200,
                        "type": "text/html; charset=utf-8",
                        "body": STANDARD.encode(self.html_pagina(pag)),
                    }))
                }
                "download" => {
                    let pag = *self.ultima_pagina.borrow();
                    Ok(json!({
                        "status": 200,
                        "type": "application/vnd.ms-excel",
                        "body": STANDARD.encode(self.xls_pagina(pag)),
                    }))
                }
                "search_and_download" => {
                    let inicio = Instant::now();
                    if !self.demora.is_zero() {
                        std::thread::sleep(self.demora);
                    }
                    let pag = Self::pagina_de_pares(&payload["pairs"]);
                    *self.ultima_pagina.borrow_mut() = pag;
                    let html = self.html_pagina(pag);
                    let xls_bytes = if pag == 1 && self.xls_corrupto_p1 {
                        b"<html><table><tr><th>RIT</th><th>Nombre</th></tr><tr><td>C-999-2099</td><td>Otro</td></tr></table></html>".to_vec()
                    } else {
                        self.xls_pagina(pag)
                    };
                    let excel = if self.vacia {
                        Value::Null
                    } else {
                        json!({
                            "status": 200,
                            "type": "application/vnd.ms-excel",
                            "url": "https://familia.pjud.cl/sitfa/reportes/u1_amblistcump.xls",
                            "body": STANDARD.encode(xls_bytes),
                        })
                    };
                    self.tiempos_sad.borrow_mut().push((inicio, Instant::now()));
                    Ok(json!({
                        "html": {
                            "status": 200,
                            "type": "text/html; charset=utf-8",
                            "body": STANDARD.encode(html),
                        },
                        "excel": excel,
                    }))
                }
                otro => Err(format!("comando inesperado: {otro}")),
            }
        }
    }

    #[test]
    fn lote_multipagina_usa_search_and_download_y_pipelining() {
        let catalogo = MockMultipagina::catalogo_seguimiento();
        let plan = MockMultipagina::plan_cumplimiento(&catalogo);

        // 1) Con 3 páginas, extensión >= 2.11.0 usa `search_and_download` con `index_dom: false` y pausa adaptativa.
        let mut mock = MockMultipagina::nuevo(true, 3);
        mock.demora = Duration::from_millis(160);
        let destino = std::env::temp_dir().join(format!("flow-multipag-sad-{}", std::process::id()));
        let _ = fs::remove_dir_all(&destino);
        let mut carpeta = PathBuf::new();
        let res = ejecutar(
            &mock,
            &catalogo,
            Edicion::Integral,
            &plan,
            &destino,
            &Opciones {
                evidencia: false,
                pausa: Duration::from_millis(200),
                ..Default::default()
            },
            &AtomicBool::new(false),
            &mut |ev| {
                if let Evento::Carpeta(c) = ev {
                    carpeta = c;
                }
            },
        )
        .unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0]["paginas"], 3);
        assert_eq!(res[0]["estado"], "VALIDADA");
        let llamadas = mock.llamadas.borrow();
        let sad_calls: Vec<_> = llamadas.iter().filter(|(c, _)| c == "search_and_download").collect();
        assert_eq!(sad_calls.len(), 3, "debe usar search_and_download una vez por página");
        assert!(sad_calls.iter().all(|(_, p)| p["index_dom"] == false), "debe pedir index_dom: false");
        assert!(llamadas.iter().all(|(c, _)| c != "search" && c != "download"), "no debe llamar search/download");
        let tiempos = mock.tiempos_sad.borrow();
        assert_eq!(tiempos.len(), 3);
        // La pausa configurada (200 ms) descuenta la latencia de red (~160 ms), por lo que la brecha entre p1 y p2 es < 150 ms.
        let brecha_1_2 = tiempos[1].0.saturating_duration_since(tiempos[0].1);
        assert!(
            brecha_1_2 < Duration::from_millis(150),
            "pausa adaptativa debió descontar latencia de red (~160 ms de 200 ms): {brecha_1_2:?}"
        );
        let verif: Value = serde_json::from_slice(&fs::read(carpeta.join("verificacion 001.json")).unwrap()).unwrap();
        assert_eq!(verif["estado"], "VALIDADA");
        assert_eq!(verif["archivos"].as_array().unwrap().len(), 3);
        let _ = fs::remove_dir_all(&destino);

        // 1b) Si el worker falla validando página 1 en el pipeline, propaga el error y marca INCOMPLETA.
        let mut mock_err = MockMultipagina::nuevo(true, 3);
        mock_err.xls_corrupto_p1 = true;
        let destino_err = std::env::temp_dir().join(format!("flow-multipag-err-{}", std::process::id()));
        let _ = fs::remove_dir_all(&destino_err);
        let mut carpeta_err = PathBuf::new();
        let err = ejecutar(
            &mock_err,
            &catalogo,
            Edicion::Integral,
            &plan,
            &destino_err,
            &Opciones {
                evidencia: false,
                ..Default::default()
            },
            &AtomicBool::new(false),
            &mut |ev| {
                if let Evento::Carpeta(c) = ev {
                    carpeta_err = c;
                }
            },
        )
        .unwrap_err();
        assert!(err.contains("coinciden"), "debe propagar el error de validación de página 1: {err}");
        let verif_err: Value =
            serde_json::from_slice(&fs::read(carpeta_err.join("verificacion 001.json")).unwrap()).unwrap();
        assert_eq!(verif_err["estado"], "INCOMPLETA");
        let _ = fs::remove_dir_all(&destino_err);

        // 2) Consulta vacía sin PDF emite `comprobante_sin_registros` (V-JSON-01) con SHA-256 real del HTML.
        let mut mock_vacia = MockMultipagina::nuevo(true, 1);
        mock_vacia.vacia = true;
        let html_esperado = mock_vacia.html_pagina(1);
        let destino_vacia = std::env::temp_dir().join(format!("flow-json-sin-pdf-{}", std::process::id()));
        let _ = fs::remove_dir_all(&destino_vacia);
        let mut carpeta_vacia = PathBuf::new();
        ejecutar(
            &mock_vacia,
            &catalogo,
            Edicion::Integral,
            &plan,
            &destino_vacia,
            &Opciones {
                evidencia: false,
                ..Default::default()
            },
            &AtomicBool::new(false),
            &mut |ev| {
                if let Evento::Carpeta(c) = ev {
                    carpeta_vacia = c;
                }
            },
        )
        .unwrap();
        let verif_vacia: Value =
            serde_json::from_slice(&fs::read(carpeta_vacia.join("verificacion 001.json")).unwrap()).unwrap();
        assert_eq!(verif_vacia["estado"], "SIN_RESULTADOS");
        assert_eq!(verif_vacia["evidencia_omitida"], true);
        let comp = &verif_vacia["comprobante_sin_registros"];
        assert_eq!(comp["tipo"], "JSON_SIN_REGISTROS");
        assert_eq!(comp["sha256_html"], huella(&html_esperado));
        assert!(!comp["contexto_consulta"].as_array().unwrap().is_empty());
        let _ = fs::remove_dir_all(&destino_vacia);
    }

    #[test]
    fn lote_multipagina_cae_a_search_y_download_con_extension_antigua() {
        let catalogo = MockMultipagina::catalogo_seguimiento();
        let plan = MockMultipagina::plan_cumplimiento(&catalogo);
        let mock = MockMultipagina::nuevo(false, 3);
        let destino = std::env::temp_dir().join(format!("flow-multipag-legacy-{}", std::process::id()));
        let _ = fs::remove_dir_all(&destino);
        let mut carpeta = PathBuf::new();
        let res = ejecutar(
            &mock,
            &catalogo,
            Edicion::Integral,
            &plan,
            &destino,
            &Opciones {
                evidencia: false,
                ..Default::default()
            },
            &AtomicBool::new(false),
            &mut |ev| {
                if let Evento::Carpeta(c) = ev {
                    carpeta = c;
                }
            },
        )
        .unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0]["paginas"], 3);
        let llamadas = mock.llamadas.borrow();
        assert!(llamadas.iter().all(|(c, _)| c != "search_and_download"));
        assert_eq!(llamadas.iter().filter(|(c, _)| c == "search").count(), 3);
        assert_eq!(llamadas.iter().filter(|(c, _)| c == "download").count(), 3);
        let verif: Value = serde_json::from_slice(&fs::read(carpeta.join("verificacion 001.json")).unwrap()).unwrap();
        assert_eq!(verif["estado"], "VALIDADA");
        let _ = fs::remove_dir_all(&destino);
    }

    #[test]
    fn diagnostico_columnas_html_xls_no_filtra_datos_personales() {
        let catalogo = MockMultipagina::catalogo_seguimiento();
        let plan = MockMultipagina::plan_cumplimiento(&catalogo);
        let mut mock = MockMultipagina::nuevo(true, 1);
        mock.incluir_rut_xls = true;
        let destino = std::env::temp_dir().join(format!("flow-diag-ex3-{}", std::process::id()));
        let _ = fs::remove_dir_all(&destino);
        let mut carpeta = PathBuf::new();
        ejecutar(
            &mock,
            &catalogo,
            Edicion::Integral,
            &plan,
            &destino,
            &Opciones {
                evidencia: false,
                ..Default::default()
            },
            &AtomicBool::new(false),
            &mut |ev| {
                if let Evento::Carpeta(c) = ev {
                    carpeta = c;
                }
            },
        )
        .unwrap();
        let diag_bytes = fs::read(carpeta.join("diagnostico_columnas_html_xls.json")).unwrap();
        let diag_text = String::from_utf8(diag_bytes).unwrap();
        let diag: Value = serde_json::from_str(&diag_text).unwrap();
        assert_eq!(diag["version"], 1);
        assert_eq!(diag["pantalla"], "seguimiento");
        let c = &diag["consultas"][0];
        assert_eq!(c["pestana"], "Cumplimiento");
        assert_eq!(c["modalidad"], "Ambulatorio");
        assert_eq!(c["columnas_html"], json!(["RIT", "Nombre", "Programa"]));
        assert_eq!(c["columnas_xls"], json!(["RIT", "RUT", "Nombre", "Programa"]));
        assert_eq!(c["solo_en_xls"], json!(["RUT"]));
        assert_eq!(c["solo_en_html"], json!([]));
        for pii in [
            "C-11-2099",
            "C-12-2099",
            "Persona Secreta Ficticia",
            "11.111.111-1",
            "Programa Ficticio Norte",
            "contacto.ficticio@example.test",
        ] {
            assert!(!diag_text.contains(pii), "el diagnóstico E-X3 filtró datos de fila: {pii}");
        }
        let _ = fs::remove_dir_all(&destino);
    }
}
