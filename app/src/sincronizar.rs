//! «Actualizar desde Excel» (`nurus.personal.sync` de Flow Python): trae a Trabajo la revisión hecha en la copia de
//! Excel. Las filas se asocian por identidad, nunca por posición; una edición hecha aquí y otra en Excel sobre el mismo
//! campo es un conflicto que la persona resuelve, y nada se aplica hasta que no hay conflictos.

use crate::config::Config;
use crate::copia_excel::identificadores;
use crate::lectura;
use crate::producto_excel;
use crate::trabajo::{Registro, Trabajo};
use crate::util::{historical_match, normalizar, Cell};
use std::collections::{BTreeMap, HashMap, HashSet};

pub const CAMPOS: [&str; 5] = ["OBSERVACION", "FECHA_OBS", "TT", "CC", "RES"];
const IDENTIDAD: [&str; 5] = ["rit", "rut", "nombre", "tribunal", "programa"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflicto {
    /// Posición de la fila en Trabajo.
    pub fila: usize,
    pub campo: String,
    pub base: String,
    pub local: String,
    pub entrante: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// La copia no corresponde al trabajo; no se aplicó nada.
    Rechazo(String),
    /// Hay campos editados aquí y en Excel; no se aplicó nada.
    Conflictos(Vec<Conflicto>),
}

impl Error {
    pub fn mensaje(&self) -> String {
        match self {
            Error::Rechazo(m) => m.clone(),
            Error::Conflictos(c) => {
                format!("Hay {} campos editados aquí y en Excel. Resuelve las diferencias; no se aplicó ningún cambio.", c.len())
            }
        }
    }
}

/// Fila de Trabajo, valores a aplicar, revisión que ve Excel y cambios en `NURUS_DECISIONES`.
struct Cambio {
    fila: usize,
    fusion: BTreeMap<&'static str, String>,
    excel: BTreeMap<String, String>,
    decisiones: Option<Option<serde_json::Value>>,
    raw_entrante: Option<String>,
}

/// Elecciones de la persona ante un conflicto: (fila, campo) → valor.
pub type Resoluciones = BTreeMap<(usize, String), String>;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Resumen {
    pub filas_cambiadas: usize,
    /// Filas con datos de origen distintos en Excel, cuya propuesta y reglas se recalcularon.
    pub recalculadas: usize,
    pub avisos: Vec<String>,
}

fn rechazo(m: &str) -> Error {
    Error::Rechazo(m.into())
}

/// Revisión que Excel tiene en una fila: los cinco campos, vacíos si no están.
fn revision_excel(cols: &HashMap<&str, usize>, fila: &[Cell]) -> BTreeMap<String, String> {
    CAMPOS
        .iter()
        .map(|c| (c.to_string(), cols.get(c).and_then(|&i| fila.get(i)).map(|v| v.display().trim().to_string()).unwrap_or_default()))
        .collect()
}

/// Revisión local de una fila con el mismo vocabulario que Excel. `None` en un campo: no hay edición local.
fn revision_local(r: &Registro) -> BTreeMap<&'static str, Option<String>> {
    let mut m = BTreeMap::new();
    m.insert("OBSERVACION", r.editada.clone());
    m.insert("RES", Some(r.res.clone()));
    for c in ["FECHA_OBS", "TT", "CC"] {
        m.insert(c, r.campos.get(c).cloned());
    }
    m
}

/// Codifica el estado completo del registro en la celda técnica `NURUS_DECISIONES` (`record_edits.encode` de Python).
pub fn codificar_decisiones(r: &Registro) -> String {
    let mut decisions = serde_json::Map::new();
    decisions.insert("structured".into(), serde_json::Value::Bool(true));
    let mail_val: serde_json::Value = serde_json::to_value(&r.gestiones_correo).unwrap_or_else(|_| serde_json::json!({}));
    decisions.insert("mail".into(), mail_val);
    let resolucion = crate::productos::modo_resolucion(r);
    decisions.insert("resolution".into(), serde_json::Value::String(resolucion.into()));

    let mut obj = serde_json::Map::new();
    obj.insert("decisions".into(), serde_json::Value::Object(decisions));
    obj.insert("excluded".into(), serde_json::Value::Bool(r.excluido));
    let overrides_val: serde_json::Value = serde_json::to_value(&r.anulaciones).unwrap_or_else(|_| serde_json::json!({}));
    obj.insert("overrides".into(), overrides_val);
    obj.insert("proposal".into(), serde_json::Value::String(r.propuesta.clone()));
    obj.insert("resolution_cell".into(), serde_json::Value::String(r.res.clone()));
    obj.insert("version".into(), serde_json::Value::Number(1.into()));
    obj.insert("encoder".into(), serde_json::json!("flow-rust-v1"));
    obj.insert("word_overrides".into(), serde_json::json!({}));

    serde_json::to_string(&serde_json::Value::Object(obj)).unwrap_or_default()
}

/// `record_edits.decode`: la celda técnica `NURUS_DECISIONES` de la copia. `Ok(None)` si está vacía; el error es el de
/// Python. Valida la estructura y tipos exactos.
pub fn decodificar_decisiones(texto: &str) -> Result<Option<serde_json::Value>, Error> {
    let invalida = || rechazo("La columna técnica NURUS_DECISIONES no es válida. No se importaron cambios.");
    let t = texto.trim();
    if t.is_empty() {
        return Ok(None);
    }
    if t.encode_utf16().count() > 32_000 { return Err(invalida()); }
    let datos: serde_json::Value = serde_json::from_str(t).map_err(|_| invalida())?;
    let Some(obj) = datos.as_object() else { return Err(invalida()); };
    if datos["version"] != 1 {
        return Err(invalida());
    }
    let Some(decisions) = obj.get("decisions").and_then(serde_json::Value::as_object) else { return Err(invalida()); };
    let Some(overrides_obj) = obj.get("overrides").and_then(serde_json::Value::as_object) else { return Err(invalida()); };
    if !datos["word_overrides"].is_object() || !datos["excluded"].is_boolean() {
        return Err(invalida());
    }
    let resolucion = decisions.get("resolution").and_then(serde_json::Value::as_str).unwrap_or("auto");
    if !["auto", "review", "none", "PC_IE", "PC_INFO", "NOMENCL"].contains(&resolucion) {
        return Err(invalida());
    }
    let mail_obj = match decisions.get("mail") {
        None => None,
        Some(m) => Some(m.as_object().ok_or_else(invalida)?),
    };
    if let Some(m) = mail_obj {
        if m.values().any(|v| !matches!(v.as_str(), Some("auto" | "include" | "omit"))) {
            return Err(invalida());
        }
    }
    if overrides_obj.values().any(|v| !v.is_string() && !v.is_number() && !v.is_boolean()) {
        return Err(invalida());
    }
    Ok(Some(datos))
}

/// Compara libros v1 previos: el codificador antiguo omitía `structured` y `mail`,
/// y representaba RES tipado como tipo en vez de review.
fn equivalente(a: &Option<serde_json::Value>, b: &Option<serde_json::Value>) -> bool {
    fn normalizado(v: &serde_json::Value, compat: bool) -> serde_json::Value {
        let mut v = v.clone();
        let tipo = v["resolution_cell"].as_str().and_then(crate::productos::res_tipo);
        let con_celda = v["resolution_cell"].as_str().is_some_and(|s| !s.trim().is_empty());
        v.as_object_mut().expect("objeto validado").remove("encoder");
        let d = v["decisions"].as_object_mut().expect("decisiones validadas");
        let heredado = d.get("structured").and_then(serde_json::Value::as_bool) != Some(true);
        d.insert("structured".into(), serde_json::json!(true));
        let modo = d.get("resolution").and_then(serde_json::Value::as_str).unwrap_or("auto");
        let modo = if compat && modo == "review" { tipo.unwrap_or("review") }
            else if compat && heredado && modo == "auto" && con_celda { tipo.unwrap_or("review") }
            else { modo }.to_string();
        d.insert("resolution".into(), serde_json::json!(modo));
        let mail = d.entry("mail").or_insert_with(|| serde_json::json!({}));
        if let Some(m) = mail.as_object_mut() { m.retain(|_, v| v.as_str() != Some("auto")); }
        v
    }
    let es_canonico = |v: &Option<serde_json::Value>| v.as_ref().is_some_and(|d| d["encoder"] == "flow-rust-v1");
    let compat = !es_canonico(a) || !es_canonico(b);
    a.as_ref().map(|v| normalizado(v, compat)) == b.as_ref().map(|v| normalizado(v, compat))
}

pub(crate) fn validar_longitud_decisiones(w: &Trabajo) -> Result<(), String> {
    for r in &w.registros {
        if codificar_decisiones(r).encode_utf16().count() > 32_000 {
            return Err(format!("NURUS_DECISIONES supera 32.000 caracteres en fila {}. No se exportó el libro.", r.fila_excel));
        }
    }
    Ok(())
}

fn validar_anulaciones(w: &Trabajo, datos: &Option<serde_json::Value>) -> Result<(), Error> {
    if let Some(d) = datos {
        if d["overrides"].as_object().is_some_and(|m| m.keys().any(|k| !w.cols.contains_key(k.as_str()))) {
            return Err(rechazo("NURUS_DECISIONES contiene una columna de origen desconocida. No se importaron cambios."));
        }
    }
    Ok(())
}

/// `record_edits.restore`: restaura las decisiones, anulaciones y estado de la fila desde el JSON decodificado.
pub fn restaurar_decisiones(r: &mut Registro, data: &serde_json::Value) {
    if let Some(decisions) = data.get("decisions").and_then(serde_json::Value::as_object) {
        if let Some(res) = decisions.get("resolution").and_then(serde_json::Value::as_str) {
            r.resolucion = res.to_string();
        }
        if let Some(mail) = decisions.get("mail").and_then(serde_json::Value::as_object) {
            r.gestiones_correo = mail
                .iter()
                .filter_map(|(k, v)| v.as_str().filter(|v| *v != "auto").map(|v| (k.clone(), v.to_string())))
                .collect();
        } else {
            r.gestiones_correo.clear();
        }
    }
    if let Some(overrides) = data.get("overrides").and_then(serde_json::Value::as_object) {
        r.anulaciones = overrides
            .iter()
            .map(|(k, v)| {
                let s = match v.as_str() {
                    Some(s) => s.to_string(),
                    None => v.to_string(),
                };
                (k.clone(), s)
            })
            .collect();
    }
    if let Some(excluded) = data.get("excluded").and_then(serde_json::Value::as_bool) {
        r.excluido = excluded;
    }
    if let Some(prop) = data.get("proposal").and_then(serde_json::Value::as_str) {
        r.propuesta = prop.to_string();
    }
    let res_cell = data.get("resolution_cell").and_then(serde_json::Value::as_str).unwrap_or("");
    if r.res.trim() != res_cell.trim() {
        r.resolucion = "review".to_string();
    }
}

fn pos(headers: &[String], titulo: &str) -> Option<usize> {
    let k = normalizar(titulo);
    headers.iter().position(|h| normalizar(h) == k)
}

fn columnas(headers: &[String]) -> HashMap<&'static str, usize> {
    CAMPOS.iter().filter_map(|c| pos(headers, c).map(|i| (*c, i))).collect()
}

/// Proyecta las celdas de revisión de la copia preservada según las mismas instrucciones que recibe Excel.
/// Las escrituras con `conservar` respetan la celda preexistente, incluida una revisión de un libro anterior.
fn revision_preservada(w: &Trabajo, base: &crate::copia_excel::Base) -> Option<Vec<BTreeMap<String, String>>> {
    let hojas = lectura::read_sheets(&base.bytes).ok()?;
    let hoja = hojas.iter().find(|h| h.name == base.hoja)?;
    let h = lectura::headers_of(hoja.rows.get(base.encabezado.checked_sub(1)?)?);
    let cols = columnas(&h);
    let plan = crate::copia_excel::instrucciones(w, base, std::path::Path::new("origen"), std::path::Path::new("destino"));
    let mut revisiones: Vec<_> = base.filas.iter().map(|&f| hoja.rows.get(f.checked_sub(1)?).map(|fila| revision_excel(&cols, fila))).collect::<Option<_>>()?;
    let indices: HashMap<usize, usize> = base.filas.iter().enumerate().map(|(i, &f)| (f, i)).collect();
    let aplicar = |revisiones: &mut Vec<BTreeMap<String, String>>, campo: &str, lista: &serde_json::Value, conservar: bool| {
        for celda in lista.as_array().into_iter().flatten() {
            let Some(i) = celda["f"].as_u64().and_then(|f| indices.get(&(f as usize))) else { continue };
            let Some(valor) = celda["v"].as_str() else { continue };
            let entrada = revisiones[*i].entry(campo.to_string()).or_default();
            if !conservar || entrada.is_empty() { *entrada = valor.trim().to_string(); }
        }
    };
    for e in plan["escrituras"].as_array()? {
        let campo = e["titulo"].as_str()?;
        if CAMPOS.contains(&campo) { aplicar(&mut revisiones, campo, &e["celdas"], e["conservar"].as_bool().unwrap_or(false)); }
    }
    aplicar(&mut revisiones, "RES", &plan["res_sugerida"], true);
    Some(revisiones)
}

/// Deja anotado qué tenía la copia que se acaba de guardar: el identificador de cada fila y su revisión.
pub fn recordar_exportacion(w: &mut Trabajo, hoja: &str) {
    let h = producto_excel::encabezados(w);
    let cols = columnas(&h);
    let bases = crate::copia_excel::base(w).ok().filter(|b| b.hoja == hoja)
        .and_then(|b| revision_preservada(w, &b))
        .unwrap_or_else(|| producto_excel::filas(w).iter().map(|f| revision_excel(&cols, f)).collect());
    let ids = identificadores(w, hoja);
    for ((r, base), id) in w.registros.iter_mut().zip(bases).zip(ids) {
        let local = revision_local(r).into_iter().map(|(k, v)| (k.to_string(), v.unwrap_or_else(|| base.get(k).cloned().unwrap_or_default()))).collect();
        r.sync_local = Some(local);
        r.sync_base = Some(base);
        r.sync_correo = Some(serde_json::to_string(&r.gestiones_correo).unwrap_or_default());
        r.sync_anulaciones = Some(serde_json::to_string(&r.anulaciones).unwrap_or_default());
        r.sync_decisiones = Some(codificar_decisiones(r));
        r.id_excel = id;
    }
}

fn identidad(headers: &[String], fila: &[Cell], claves: &[&str], modo: crate::motor::Mode) -> Vec<String> {
    let mapa = lectura::map_columns(headers, modo);
    claves
        .iter()
        .map(|k| mapa.get(k).and_then(|&i| fila.get(i)).map(|c| historical_match(&c.display())).unwrap_or_default())
        .collect()
}

/// `sync.associate`: una fila del trabajo por fila de la copia, validando todo el conjunto antes de devolver.
fn asociar(w: &Trabajo, headers: &[String], filas: &[Vec<Cell>]) -> Result<Vec<usize>, Error> {
    let claves: Vec<&str> = IDENTIDAD.iter().copied().filter(|k| w.cols.contains_key(k)).collect();
    if !["rit", "nombre", "tribunal"].iter().all(|k| claves.contains(k)) {
        return Err(rechazo("Faltan campos de identidad; no se aplicaron cambios."));
    }
    let modo = w.mode();
    let col_id = pos(headers, "NURUS_ID_REGISTRO");
    let por_id: HashMap<&str, usize> = w.registros.iter().enumerate().filter(|(_, r)| !r.id_excel.is_empty()).map(|(i, r)| (r.id_excel.as_str(), i)).collect();
    let mut por_identidad: HashMap<Vec<String>, Vec<usize>> = HashMap::new();
    for (i, r) in w.registros.iter().enumerate() {
        let k: Vec<String> = claves.iter().map(|c| historical_match(&w.raw_value(r, c))).collect();
        por_identidad.entry(k).or_default().push(i);
    }
    let mut resultado = Vec::with_capacity(filas.len());
    let mut vistas = HashSet::new();
    for fila in filas {
        let en_excel = identidad(headers, fila, &claves, modo);
        let i = match col_id {
            Some(c) => {
                let id = fila.get(c).map(|v| v.display().trim().to_string()).unwrap_or_default();
                *por_id.get(id.as_str()).ok_or_else(|| rechazo("Hay identidades ajenas o vacías en la copia; no se aplicaron cambios."))?
            }
            None => match por_identidad.get(&en_excel).map(Vec::as_slice) {
                Some([unico]) => *unico,
                _ => return Err(rechazo("Falta la columna de identidad y hay registros ambiguos o modificados; no se aplicaron cambios.")),
            },
        };
        if !vistas.insert(i) {
            return Err(rechazo("Hay identidades duplicadas en la copia; no se aplicaron cambios."));
        }
        let local: Vec<String> = claves.iter().map(|c| historical_match(&w.raw_value(&w.registros[i], c))).collect();
        if local != en_excel {
            return Err(rechazo("Cambió la identidad de una fila; no se aplicaron cambios."));
        }
        resultado.push(i);
    }
    if vistas.len() != w.registros.len() {
        return Err(rechazo("Faltan registros en la copia; no se aplicaron cambios."));
    }
    Ok(resultado)
}

/// Resultado de combinar un campo: `Ok(Some(v))` toma `v`; `Ok(None)` conserva lo local; `Err` es un conflicto.
fn combinar(
    campo: &str,
    local: &Option<String>,
    base: Option<&String>,
    sync_local: Option<&String>,
    remoto: &str,
    con_base: bool,
) -> Result<Option<String>, (String, String, String)> {
    let base_v = base.map_or("", String::as_str);
    let local_v = local.as_deref().unwrap_or(base_v);
    let sync_local_v = sync_local.map_or(base_v, String::as_str);
    let mut local_cambio = local.is_some() && local_v != sync_local_v;
    if !con_base {
        // Sesiones sin línea base: nunca se sobrescribe en silencio un valor local.
        local_cambio = local.is_some() && local_v != remoto;
    }
    let remoto_cambio = !con_base || remoto != base_v;
    let _ = campo;
    if local_cambio && remoto_cambio && local_v != remoto {
        return Err((base_v.into(), local_v.into(), remoto.into()));
    }
    Ok(remoto_cambio.then(|| remoto.to_string()))
}

/// `sync.refresh`: combina la revisión de la copia con la de Trabajo. Todo o nada.
///
/// F10: si una columna de origen cambió en la copia, se toma el valor de Excel y se recalculan la propuesta y las reglas
/// (`record_edits.recalculate`); la revisión de la persona se conserva. En un trabajo de paquete el contexto de lectura
/// (historia, bitácora, calendario) sigue siendo el de la captura. Diferencia con Python: se vuelven a analizar todas las
/// filas, no solo las editadas, así que también se actualiza una fila cuya duplicidad depende de la editada.
pub fn actualizar(w: &mut Trabajo, cfg: &Config, headers: &[String], filas: &[Vec<Cell>], resoluciones: &Resoluciones) -> Result<Resumen, Error> {
    let asociadas = asociar(w, headers, filas)?;
    let cols = columnas(headers);
    let mut conflictos = Vec::new();
    // (fila de Trabajo, valores a aplicar, revisión que ve Excel)
    let mut cambios: Vec<Cambio> = Vec::new();
    for (&i, fila) in asociadas.iter().zip(filas) {
        let r = &w.registros[i];
        let fila_hoja = r.fila_excel.checked_sub(1).ok_or_else(|| rechazo("Fila de origen inválida."))?;
        if w.sheets.get(w.sheet).and_then(|h| h.rows.get(fila_hoja)).is_none() {
            return Err(rechazo("Fila de origen inexistente."));
        }
        let excel = revision_excel(&cols, fila);
        let local = revision_local(r);
        let mut fusion = BTreeMap::new();
        for campo in CAMPOS {
            let remoto = excel.get(campo).map_or("", String::as_str);
            let base = r.sync_base.as_ref().and_then(|b| b.get(campo));
            let sync_local = r.sync_local.as_ref().and_then(|b| b.get(campo));
            let propio = local.get(campo).cloned().flatten();
            // RES siempre tiene un valor local (la sugerencia del motor); lo demás solo si la persona lo editó.
            match combinar(campo, &propio, base, sync_local, remoto, r.sync_base.is_some()) {
                Ok(Some(v)) => {
                    fusion.insert(campo, v);
                }
                Ok(None) => {}
                Err((b, l, e)) => match resoluciones.get(&(i, campo.to_string())) {
                    Some(v) => {
                        fusion.insert(campo, v.clone());
                    }
                    None => conflictos.push(Conflicto { fila: i, campo: campo.into(), base: b, local: l, entrante: e }),
                },
            }
        }
        // Gestiones de correo, anulaciones y resoluciones (`NURUS_DECISIONES`):
        // Se compara el estado completo codificado (decisions, overrides, word_overrides, excluded, proposal, resolution_cell).
        let mut decisiones = None;
        let mut raw_entrante = None;
        if let Some(c) = pos(headers, "NURUS_DECISIONES") {
            let technical_raw = fila.get(c).map(|v| v.display().trim().to_string()).unwrap_or_default();
            raw_entrante = Some(technical_raw.clone());
            let technical_data = decodificar_decisiones(&technical_raw)?;
            let baseline_data = r.sync_decisiones.as_deref().and_then(|s| decodificar_decisiones(s).ok().flatten());
            validar_anulaciones(w, &technical_data)?;
            if technical_data.is_some() && !equivalente(&technical_data, &baseline_data) {
                let local_raw = codificar_decisiones(r);
                let local_data = decodificar_decisiones(&local_raw)?;
                let conflict = !equivalente(&local_data, &baseline_data) && !equivalente(&local_data, &technical_data);
                let mut data_to_restore = technical_data;
                if conflict {
                    match resoluciones.get(&(i, "NURUS_DECISIONES".to_string())) {
                        Some(res_str) => {
                            data_to_restore = decodificar_decisiones(res_str)?;
                            validar_anulaciones(w, &data_to_restore)?;
                        }
                        None => {
                            let base_str = r.sync_decisiones.clone().unwrap_or_default();
                            conflictos.push(Conflicto {
                                fila: i,
                                campo: "NURUS_DECISIONES".into(),
                                base: base_str,
                                local: local_raw,
                                entrante: technical_raw,
                            });
                        }
                    }
                }
                decisiones = Some(data_to_restore);
            }
        }
        cambios.push(Cambio { fila: i, fusion, excel, decisiones, raw_entrante });
    }
    if !conflictos.is_empty() {
        return Err(Error::Conflictos(conflictos));
    }
    let mut resumen = Resumen::default();
    let mut recalculadas = HashSet::new();
    let mut nuevos_valores: Vec<(usize, Vec<(usize, Cell)>)> = Vec::new();
    for (c, fila) in cambios.into_iter().zip(filas) {
        let i = c.fila;
        let r = &mut w.registros[i];
        let antes = (
            r.editada.clone(),
            r.res.clone(),
            r.campos.clone(),
            r.gestiones_correo.clone(),
            r.anulaciones.clone(),
            r.resolucion.clone(),
            r.excluido,
        );
        for (campo, v) in c.fusion {
            match campo {
                // Una observación vaciada en Excel no borra la local ni la propuesta.
                "OBSERVACION" if !v.is_empty() => r.editada = Some(v),
                "OBSERVACION" => {}
                "RES" => r.res = v,
                otro if v.is_empty() => {
                    r.campos.remove(otro);
                }
                otro => {
                    r.campos.insert(otro.to_string(), v);
                }
            }
        }
        if let Some(Some(data)) = c.decisiones {
            restaurar_decisiones(r, &data);
        }
        // Si cambió RES en Excel respecto de la línea base, la resolución pasa a "review"
        if r.sync_base.is_some() {
            let base_res = r.sync_base.as_ref().and_then(|b| b.get("RES")).map_or("", String::as_str);
            let remote_res = c.excel.get("RES").map_or("", String::as_str);
            if remote_res.trim() != base_res.trim() && r.res.trim() == remote_res.trim() {
                r.resolucion = "review".to_string();
            }
        }
        r.sync_correo = c.raw_entrante.as_deref().and_then(|raw| {
            decodificar_decisiones(raw).ok().flatten().and_then(|d| {
                d.get("decisions")
                    .and_then(|dec| dec.get("mail"))
                    .map(|m| serde_json::to_string(m).unwrap_or_default())
            })
        }).or_else(|| Some(serde_json::to_string(&r.gestiones_correo).unwrap_or_default()));
        r.sync_anulaciones = c.raw_entrante.as_deref().and_then(|raw| {
            decodificar_decisiones(raw).ok().flatten().and_then(|d| {
                d.get("overrides").map(|o| serde_json::to_string(o).unwrap_or_default())
            })
        }).or_else(|| Some(serde_json::to_string(&r.anulaciones).unwrap_or_default()));
        if let Some(raw) = c.raw_entrante {
            r.sync_decisiones = Some(raw);
        }
        if antes != (
            r.editada.clone(),
            r.res.clone(),
            r.campos.clone(),
            r.gestiones_correo.clone(),
            r.anulaciones.clone(),
            r.resolucion.clone(),
            r.excluido,
        ) {
            resumen.filas_cambiadas += 1;
        }
        r.sync_base = Some(c.excel.clone());
        r.sync_local = Some(c.excel);
        // Columnas de origen que cambiaron en la copia (sin las de identidad, que `asociar` ya validó).
        let mapa = lectura::map_columns(headers, w.mode());
        let w_ref: &Trabajo = w;
        let editadas: Vec<(usize, usize)> = w_ref
            .cols
            .iter()
            .filter(|(k, _)| !IDENTIDAD.contains(k) && **k != "res")
            .filter_map(|(k, &c)| {
                let j = *mapa.get(k)?;
                let en_trabajo = w_ref.registros[i].values.get(c).map(Cell::display).unwrap_or_default();
                let en_excel = fila.get(j).map(Cell::display).unwrap_or_default();
                (normalizar(en_trabajo.trim()) != normalizar(en_excel.trim())).then_some((c, j))
            })
            .collect();
        if !editadas.is_empty() {
            recalculadas.insert(i);
            nuevos_valores.push((i, editadas.into_iter().map(|(c, j)| (c, fila.get(j).cloned().unwrap_or(Cell::Empty))).collect()));
        }
    }
    if !recalculadas.is_empty() {
        let hoja = w.sheet;
        for (i, valores) in nuevos_valores {
            let fila_hoja = w.registros[i].fila_excel.checked_sub(1).expect("fila validada");
            let row = &mut w.sheets[hoja].rows[fila_hoja];
            for (c, v) in valores {
                if c >= row.len() { row.resize(c + 1, Cell::Empty); }
                row[c] = v;
            }
            recalculadas.insert(i);
        }
        let exclusiones: HashMap<usize, bool> = w.registros.iter().filter(|r| crate::conciliar::exclusion_manual(r)).map(|r| (r.fila_excel, r.excluido)).collect();
        let res: HashMap<usize, String> = w.registros.iter().map(|r| (r.fila_excel, r.res.clone())).collect();
        w.analizar(cfg);
        for r in &mut w.registros {
            if let Some(excluido) = exclusiones.get(&r.fila_excel) { r.excluido = *excluido; }
            if let Some(v) = res.get(&r.fila_excel) { r.res = v.clone(); }
        }
        resumen.recalculadas = recalculadas.len();
    }
    Ok(resumen)
}

/// Lee la copia guardada: la hoja de datos (la del modo o la que trae `NURUS_ID_REGISTRO`) con su encabezado.
pub fn leer_copia(w: &Trabajo, bytes: &[u8]) -> Result<(Vec<String>, Vec<Vec<Cell>>), String> {
    let hojas = lectura::read_sheets(bytes).map_err(|_| "No se pudo leer la copia de Excel.".to_string())?;
    let modo = w.mode().label().to_uppercase();
    let encabezado = |h: &lectura::Sheet| {
        h.rows.iter().take(15).position(|f| {
            let t: Vec<String> = f.iter().map(|c| normalizar(&c.display())).collect();
            let hay = |n: &str| t.contains(&normalizar(n));
            hay("RIT") && (hay("OBSERVACION") || hay("NURUS_ID_REGISTRO"))
        })
    };
    let hoja = hojas
        .iter()
        .find(|h| h.name.to_uppercase() == modo && encabezado(h).is_some())
        .or_else(|| hojas.iter().find(|h| encabezado(h).is_some()))
        .ok_or("La copia no tiene una hoja de datos reconocible.")?;
    let inicio = encabezado(hoja).unwrap();
    let headers = lectura::headers_of(&hoja.rows[inicio]);
    let filas = hoja.rows[inicio + 1..]
        .iter()
        .filter(|f| f.iter().any(|c| !c.is_empty()))
        .map(|f| f.to_vec())
        .collect();
    Ok((headers, filas))
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::config::Config;
    use crate::motor::Mode;
    use chrono::NaiveDate;
    use serde_json::Value;
    use std::path::PathBuf;

    #[test]
    fn decisiones_v1_compatibles_y_modos_canonicos_distintos() {
        let mut r = Registro { res: "PC_IE".into(), ..Default::default() };
        let canon = decodificar_decisiones(&codificar_decisiones(&r)).unwrap();
        let mut viejo = canon.clone().unwrap();
        viejo.as_object_mut().unwrap().remove("encoder");
        viejo["decisions"].as_object_mut().unwrap().remove("structured");
        viejo["decisions"].as_object_mut().unwrap().remove("mail");
        assert!(equivalente(&canon, &Some(viejo.clone())));
        viejo["decisions"]["structured"] = serde_json::json!(true);
        viejo["decisions"]["resolution"] = serde_json::json!("review");
        assert!(equivalente(&canon, &Some(viejo)));
        r.resolucion = "review".into();
        assert!(!equivalente(&canon, &decodificar_decisiones(&codificar_decisiones(&r)).unwrap()));
        r.resolucion = "none".into();
        let none = decodificar_decisiones(&codificar_decisiones(&r)).unwrap().unwrap();
        assert_eq!(none["decisions"]["resolution"], "none");
    }

    #[test]
    fn copia_preservada_plan_intacto_y_conflicto_real() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/sync.json")).unwrap();
        let mut w = trabajo(&datos);
        w.registros[0].revisada = true;
        w.registros[0].editada = Some("Constancia sintética".into());
        w.registros[0].campos.insert("TT".into(), "1".into());
        w.registros[1].resolucion = "none".into();
        w.registros[2].res = "PC_IE".into();
        let base = crate::copia_excel::base(&w).unwrap();
        let plan = crate::copia_excel::instrucciones(&w, &base, std::path::Path::new("origen"), std::path::Path::new("destino"));
        let h = producto_excel::encabezados(&w);
        let mut filas: Vec<Vec<Cell>> = w.registros.iter().map(|r| {
            let mut f = vec![Cell::Empty; h.len()];
            for (t, v) in w.headers.iter().zip(&r.values) { if let Some(c) = pos(&h, t) { f[c] = v.clone(); } }
            f
        }).collect();
        // Simula Escribir de PowerShell sobre los datos originales; no ejecuta Excel/COM.
        for e in plan["escrituras"].as_array().unwrap() {
            let c = pos(&h, e["titulo"].as_str().unwrap()).unwrap();
            for v in e["celdas"].as_array().unwrap() {
                let i = base.filas.iter().position(|f| *f == v["f"].as_u64().unwrap() as usize).unwrap();
                if e["conservar"] == false || filas[i][c].is_empty() { filas[i][c] = Cell::Text(v["v"].as_str().unwrap().into()); }
            }
        }
        for v in plan["res_sugerida"].as_array().unwrap() {
            let i = base.filas.iter().position(|f| *f == v["f"].as_u64().unwrap() as usize).unwrap();
            let c = pos(&h, "RES").unwrap();
            if filas[i][c].is_empty() { filas[i][c] = Cell::Text(v["v"].as_str().unwrap().into()); }
        }
        recordar_exportacion(&mut w, &base.hoja);
        let r = actualizar(&mut w, &Config::default(), &h, &filas, &Resoluciones::new()).unwrap();
        assert_eq!((r.filas_cambiadas, r.recalculadas), (0, 0));
        w.registros[0].editada = Some("Local sintético".into());
        filas[0][pos(&h, "OBSERVACION").unwrap()] = Cell::Text("Remoto sintético".into());
        assert!(matches!(actualizar(&mut w, &Config::default(), &h, &filas, &Resoluciones::new()), Err(Error::Conflictos(c)) if c.iter().any(|c| c.campo == "OBSERVACION")));
    }

    #[test]
    fn overrides_remotos_se_restauran_y_clave_invalida_no_muta() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/sync.json")).unwrap();
        let mut w = trabajo(&datos);
        recordar_exportacion(&mut w, "ESPERA");
        let h = producto_excel::encabezados(&w);
        let mut filas = producto_excel::filas(&w);
        let c = pos(&h, "NURUS_DECISIONES").unwrap();
        let mut d = decodificar_decisiones(&codificar_decisiones(&w.registros[0])).unwrap().unwrap();
        d["overrides"]["desconocida"] = serde_json::json!("5");
        filas[0][c] = Cell::Text(d.to_string());
        assert!(matches!(actualizar(&mut w, &Config::default(), &h, &filas, &Resoluciones::new()), Err(Error::Rechazo(_))));
        assert!(w.registros[0].anulaciones.is_empty());
        d["overrides"] = serde_json::json!({"espera":"5"});
        filas[0][c] = Cell::Text(d.to_string());
        let r = actualizar(&mut w, &Config::default(), &h, &filas, &Resoluciones::new()).unwrap();
        assert_eq!(r.recalculadas, 0);
        assert_eq!(w.registros[0].anulaciones["espera"], "5");
        // Al editar además una columna de origen, el recálculo respeta el override restaurado.
        filas[0][pos(&h, "T ESPERA").unwrap()] = Cell::Text("200".into());
        let antes = w.registros[0].propuesta.clone();
        let r = actualizar(&mut w, &Config::default(), &h, &filas, &Resoluciones::new()).unwrap();
        assert_eq!(r.recalculadas, 1);
        assert_ne!(w.registros[0].propuesta, antes);
    }

    #[test]
    fn decisiones_demasiado_largas_rechazadas() {
        let w = Trabajo { registros: vec![Registro { propuesta: "a".repeat(32_001), ..Default::default() }], ..Default::default() };
        assert!(validar_longitud_decisiones(&w).is_err());
        assert!(decodificar_decisiones(&codificar_decisiones(&w.registros[0])).is_err());
    }

    #[test]
    fn fila_corta_se_amplia_y_fila_cero_se_rechaza() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/sync.json")).unwrap();
        let mut w = trabajo(&datos);
        let col = w.cols["espera"];
        let f = w.registros[0].fila_excel - 1;
        w.sheets[w.sheet].rows[f].truncate(col);
        w.registros[0].values.truncate(col);
        recordar_exportacion(&mut w, "ESPERA");
        let h = producto_excel::encabezados(&w);
        let mut filas = producto_excel::filas(&w);
        filas[0][pos(&h, "T ESPERA").unwrap()] = Cell::Text("150".into());
        let r = actualizar(&mut w, &Config::default(), &h, &filas, &Resoluciones::new()).unwrap();
        assert_eq!(r.recalculadas, 1);
        assert_eq!(w.sheets[w.sheet].rows[f][col].display(), "150");
        w.registros[0].fila_excel = 0;
        assert!(matches!(actualizar(&mut w, &Config::default(), &h, &filas, &Resoluciones::new()), Err(Error::Rechazo(_))));
    }

    /// El libro de origen que armó Python (título, fila vacía, encabezado y tres filas), escrito aquí con los mismos datos.
    fn trabajo(datos: &Value) -> Trabajo {
        let mut libro = rust_xlsxwriter::Workbook::new();
        let hoja = libro.add_worksheet();
        hoja.set_name("informe_1").unwrap();
        hoja.write_string(0, 0, "Reporte RUS").unwrap();
        for (c, h) in datos["encabezados"].as_array().unwrap().iter().enumerate() {
            hoja.write_string(2, c as u16, h.as_str().unwrap()).unwrap();
        }
        for (i, fila) in datos["filas"].as_array().unwrap().iter().enumerate() {
            for (c, v) in fila.as_array().unwrap().iter().enumerate() {
                match v.as_f64() {
                    Some(n) => hoja.write_number(3 + i as u32, c as u16, n).unwrap(),
                    None => hoja.write_string(3 + i as u32, c as u16, v.as_str().unwrap()).unwrap(),
                };
            }
        }
        let bytes = libro.save_to_buffer().unwrap();
        let corte = NaiveDate::parse_from_str(datos["corte"].as_str().unwrap(), "%Y-%m-%d").unwrap();
        Trabajo::abrir(PathBuf::from("origen.xlsx"), bytes, Some(Mode::Espera), corte, &Config::default()).unwrap()
    }

    fn texto(v: &Value) -> String {
        v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())
    }

    /// Repite el escenario de Python sobre Trabajo y devuelve las diferencias con lo que Python grabó.
    fn comparar(datos: &Value, caso: &Value) -> Vec<String> {
        comparar_con(trabajo(datos), "ESPERA", caso)
    }

    /// Repite el escenario sobre un trabajo ya armado (de un archivo de RUS o de un paquete); `hoja` es la de su copia.
    fn comparar_con(mut w: Trabajo, hoja: &str, caso: &Value) -> Vec<String> {
        recordar_exportacion(&mut w, hoja);
        let sugerida: Vec<String> = w.registros.iter().map(|r| r.res.clone()).collect();
        for l in caso["locales"].as_array().unwrap() {
            let (i, campo, v) = (l[0].as_u64().unwrap() as usize, l[1].as_str().unwrap(), texto(&l[2]));
            let r = &mut w.registros[i];
            match campo {
                "OBSERVACION" => r.editada = Some(v),
                "RES" => r.res = v,
                otro => {
                    r.campos.insert(otro.into(), v);
                }
            }
        }
        for c in caso["correos_locales"].as_array().into_iter().flatten() {
            let mapa = c[1].as_object().unwrap().iter().map(|(k, v)| (k.clone(), texto(v))).collect();
            w.registros[c[0].as_u64().unwrap() as usize].gestiones_correo = mapa;
        }
        for a in caso["anulaciones_locales"].as_array().into_iter().flatten() {
            let fila = a[0].as_u64().unwrap() as usize;
            let mapa = a[1].as_object().unwrap().iter().map(|(k, v)| (k.clone(), texto(v))).collect();
            w.aplicar_anulaciones(fila, mapa, &Config::default()).unwrap();
        }
        let mut headers = producto_excel::encabezados(&w);
        let mut filas = producto_excel::filas(&w);
        // Si no se pasaron remotas de NURUS_DECISIONES, la copia de Excel conserva la exportada inicialmente
        let col_dec = pos(&headers, "NURUS_DECISIONES").unwrap();
        for (i, r) in w.registros.iter().enumerate() {
            filas[i][col_dec] = Cell::Text(codificar_decisiones(r));
        }
        if caso.get("sesion_antigua").and_then(Value::as_bool) == Some(true) {
            for r in &mut w.registros {
                r.sync_base = None;
                r.sync_local = None;
                r.sync_decisiones = None;
                r.sync_correo = None;
                r.sync_anulaciones = None;
            }
        }
        for e in caso["remotas"].as_array().unwrap() {
            let (i, campo, v) = (e[0].as_u64().unwrap() as usize, e[1].as_str().unwrap(), texto(&e[2]));
            let c = pos(&headers, campo).unwrap();
            filas[i][c] = if e[2].is_number() { Cell::Number(e[2].as_f64().unwrap()) } else { Cell::Text(v) };
        }
        let excel = &caso["excel"];
        if let Some(i) = excel.get("cambiar_identidad").and_then(Value::as_u64) {
            let c = pos(&headers, "NOMBRE").unwrap();
            filas[i as usize][c] = Cell::Text("Otra Persona Ficticia".into());
        }
        if let Some(n) = excel.get("quitar_filas").and_then(Value::as_u64) {
            filas.truncate(filas.len() - n as usize);
        }
        if excel.get("quitar_id").and_then(Value::as_bool) == Some(true) {
            let c = pos(&headers, "NURUS_ID_REGISTRO").unwrap();
            headers.remove(c);
            filas.iter_mut().for_each(|f| {
                f.remove(c);
            });
        }
        let resoluciones: Resoluciones = caso["resoluciones"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| ((r[0].as_u64().unwrap() as usize, r[1].as_str().unwrap().to_string()), texto(&r[2])))
            .collect();
        let resultado = actualizar(&mut w, &Config::default(), &headers, &filas, &resoluciones);
        let mut d = Vec::new();
        match (&resultado, caso.get("error"), caso.get("conflictos")) {
            (Err(Error::Rechazo(m)), Some(e), _) if *m == texto(e) => {}
            (Err(Error::Conflictos(c)), None, Some(esperados)) => {
                // La celda técnica trae todo el estado de la fila en Python: solo se compara qué fila y qué campo chocan.
                let tecnica = |x: &Conflicto, v: &String| if x.campo == "NURUS_DECISIONES" { String::new() } else { v.clone() };
                let mut obtenidos: Vec<Vec<String>> = c
                    .iter()
                    .map(|x| vec![x.fila.to_string(), x.campo.clone(), tecnica(x, &x.base), tecnica(x, &x.local), tecnica(x, &x.entrante)])
                    .collect();
                obtenidos.sort();
                let mut previstos: Vec<Vec<String>> = esperados
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|x| x.as_array().unwrap().iter().map(texto).collect())
                    .collect();
                previstos.sort();
                if obtenidos != previstos {
                    d.push(format!("conflictos Rust {obtenidos:?} · Python {previstos:?}"));
                }
            }
            (Ok(_), None, None) => {}
            (otro, e, c) => d.push(format!("resultado Rust {otro:?} · Python error {e:?} conflictos {c:?}")),
        }
        // F10: lo que Python recalcula al cambiar una columna de origen (y lo que deja igual en las demás filas).
        for (i, propuesta) in caso["propuestas"].as_array().unwrap().iter().enumerate() {
            let r = &w.registros[i];
            let reglas: Vec<String> = caso["reglas"][i].as_array().unwrap().iter().map(texto).collect();
            if r.eventos != reglas {
                d.push(format!("fila {i} reglas: Rust {:?} · Python {reglas:?}", r.eventos));
            }
            // R6 (decisión del 20-nov): el nombre del programa en tipo título («Dos», no «DOS»).
            let esperada = texto(propuesta).replace("AFT Ejemplo DOS", "AFT Ejemplo Dos");
            if r.propuesta != esperada {
                d.push(format!("fila {i} propuesta: Rust {:?} · Python {esperada:?}", r.propuesta));
            }
        }
        for (i, esperado) in caso["correo"].as_array().into_iter().flatten().enumerate() {
            let previsto: BTreeMap<String, String> = esperado.as_object().unwrap().iter().map(|(k, v)| (k.clone(), texto(v))).collect();
            if w.registros[i].gestiones_correo != previsto {
                d.push(format!("fila {i} gestiones de correo: Rust {:?} · Python {previsto:?}", w.registros[i].gestiones_correo));
            }
        }
        for (i, esperado) in caso["anulaciones"].as_array().into_iter().flatten().enumerate() {
            let previsto: BTreeMap<String, String> = esperado.as_object().unwrap().iter().map(|(k, v)| (k.clone(), texto(v))).collect();
            if w.registros[i].anulaciones != previsto {
                d.push(format!("fila {i} anulaciones: Rust {:?} · Python {previsto:?}", w.registros[i].anulaciones));
            }
        }
        for (i, esperado) in caso.get("resolucion").and_then(Value::as_array).into_iter().flatten().enumerate() {
            let previsto = texto(esperado);
            let obtenido = if w.registros[i].resolucion.is_empty() { "auto" } else { &w.registros[i].resolucion };
            if obtenido != previsto {
                d.push(format!("fila {i} resolución: Rust {obtenido:?} · Python {previsto:?}"));
            }
        }
        for (i, esperada) in caso["revision"].as_array().unwrap().iter().enumerate() {
            let r = &w.registros[i];
            for campo in CAMPOS {
                let obtenido = match campo {
                    "OBSERVACION" => r.editada.clone().unwrap_or_default(),
                    "RES" if r.res == sugerida[i] => String::new(),
                    "RES" => r.res.clone(),
                    otro => r.campos.get(otro).cloned().unwrap_or_default(),
                };
                let previsto = texto(&esperada[campo]);
                if obtenido != previsto {
                    d.push(format!("fila {i} {campo}: Rust {obtenido:?} · Python {previsto:?}"));
                }
            }
        }
        d
    }

    /// E-A1: «Actualizar desde Excel» de Rust contra `nurus.personal.sync.refresh` de Flow Python
    /// (`app/qa/generar_sync.py`): la revisión de cada fila, los conflictos y los rechazos con su mensaje.
    #[test]
    fn actualizar_desde_excel_igual_a_python() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/sync.json")).unwrap();
        let casos = datos["casos"].as_array().unwrap();
        let (mut iguales, mut fallos) = (0, Vec::new());
        for caso in casos {
            let d = comparar(&datos, caso);
            if d.is_empty() {
                iguales += 1;
            } else {
                fallos.push(format!("{}:\n  {}", caso["escenario"], d.join("\n  ")));
            }
        }
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("SYNC: {iguales}/{}, fallos {}", casos.len(), fallos.len());
        assert_eq!(casos.len(), 38, "el oráculo debe traer 38 casos");
        assert!(fallos.is_empty());
    }

    /// Prueba permanente del oráculo: si Python hubiera grabado otra revisión, la comparación debe fallar.
    #[test]
    fn oraculo_de_sync_detecta_alteraciones() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/sync.json")).unwrap();
        let mut caso = datos["casos"].as_array().unwrap().iter().find(|c| c["escenario"] == "remota").unwrap().clone();
        assert!(comparar(&datos, &caso).is_empty());
        caso["revision"][0]["CC"] = Value::String("0".into());
        assert!(!comparar(&datos, &caso).is_empty(), "un valor esperado alterado no se detecta");
        // También la propuesta que Python recalculó al editar una columna de origen.
        let mut recalculo = datos["casos"].as_array().unwrap().iter().find(|c| c["escenario"] == "origen_editado").unwrap().clone();
        assert!(comparar(&datos, &recalculo).is_empty());
        recalculo["reglas"][1] = serde_json::json!(["ESPERA.E05_PROYECTO_Y_CORREO"]);
        assert!(!comparar(&datos, &recalculo).is_empty(), "una regla recalculada alterada no se detecta");
        // Y la gestión de correo que Python combinó desde la celda técnica.
        let mut correo = datos["casos"].as_array().unwrap().iter().find(|c| c["escenario"] == "correo_remoto").unwrap().clone();
        assert!(comparar(&datos, &correo).is_empty());
        correo["correo"][0] = serde_json::json!({"medidas": "include"});
        assert!(!comparar(&datos, &correo).is_empty(), "una gestión de correo esperada alterada no se detecta");
        // Y la anulación de origen que Python combinó desde la celda técnica.
        let mut anul = datos["casos"].as_array().unwrap().iter().find(|c| c["escenario"] == "anulacion_remota_y_origen_editado_en_excel").unwrap().clone();
        assert!(comparar(&datos, &anul).is_empty());
        anul["anulaciones"][0] = serde_json::json!({"espera": "5"});
        assert!(!comparar(&datos, &anul).is_empty(), "una anulación esperada alterada no se detecta");
        // E-A11: y el estado combinado de resolución que Python combinó desde la celda técnica.
        let mut res_rem = datos["casos"].as_array().unwrap().iter().find(|c| c["escenario"] == "resolucion_remota").unwrap().clone();
        assert!(comparar(&datos, &res_rem).is_empty());
        res_rem["resolucion"][0] = serde_json::json!("PC_IE");
        assert!(!comparar(&datos, &res_rem).is_empty(), "una resolución esperada alterada no se detecta");
    }

    /// El archivo que guarda Flow Rust se puede volver a leer y a importar sin cambios ni conflictos.
    #[test]
    fn la_copia_guardada_se_importa_sin_cambios() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/sync.json")).unwrap();
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let mut w = trabajo(&datos);
        recordar_exportacion(&mut w, "ESPERA");
        let ruta = producto_excel::exportar(&w, &perfil.join("sync"), None).unwrap();
        let (headers, filas) = leer_copia(&w, &std::fs::read(&ruta).unwrap()).unwrap();
        assert_eq!(filas.len(), 3);
        let resumen = actualizar(&mut w, &Config::default(), &headers, &filas, &Resoluciones::new()).unwrap();
        assert_eq!(resumen.filas_cambiadas, 0, "{resumen:?}");
        assert!(resumen.avisos.is_empty(), "{:?}", resumen.avisos);
    }

    /// Un conflicto en las gestiones de correo se resuelve con el valor que elige la persona (el texto que muestra el
    /// diálogo: el de aquí o el de Excel) y la línea base pasa a la de Excel.
    #[test]
    fn un_conflicto_de_gestion_de_correo_se_resuelve_con_el_valor_elegido() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/sync.json")).unwrap();
        let celda = |mail: &str| {
            Cell::Text(format!(r#"{{"version":1,"decisions":{{"mail":{{"medidas":"{mail}"}}}},"overrides":{{}},"word_overrides":{{}},"excluded":false}}"#))
        };
        for (elegido, esperado) in [("local", "omit"), ("excel", "include")] {
            let mut w = trabajo(&datos);
            recordar_exportacion(&mut w, "ESPERA");
            w.registros[0].gestiones_correo.insert("medidas".into(), "omit".into());
            let headers = producto_excel::encabezados(&w);
            let mut filas = producto_excel::filas(&w);
            let c = pos(&headers, "NURUS_DECISIONES").unwrap();
            filas[0][c] = celda("include");
            let Err(Error::Conflictos(conflictos)) = actualizar(&mut w, &Config::default(), &headers, &filas, &Resoluciones::new()) else {
                panic!("debía haber un conflicto");
            };
            assert_eq!((conflictos.len(), conflictos[0].campo.as_str()), (1, "NURUS_DECISIONES"));
            assert_eq!(w.registros[0].gestiones_correo["medidas"], "omit", "sin resolver no se aplica nada");
            let texto = if elegido == "local" { conflictos[0].local.clone() } else { conflictos[0].entrante.clone() };
            let resoluciones = Resoluciones::from([((0, "NURUS_DECISIONES".to_string()), texto)]);
            actualizar(&mut w, &Config::default(), &headers, &filas, &resoluciones).unwrap();
            assert_eq!(w.registros[0].gestiones_correo["medidas"], esperado, "{elegido}");
            assert_eq!(w.registros[0].sync_correo.as_deref(), Some(r#"{"medidas":"include"}"#), "la línea base es la de Excel");
        }
    }

    /// E-A9: «Actualizar desde Excel» en un trabajo que viene de un paquete, contra `refresh` de Flow Python
    /// (`app/qa/generar_sync_paquete.py`): el lote de listados con tres columnas de origen, empaquetado por cada lado.
    fn trabajo_de_paquete(datos: &Value, carpeta: &std::path::Path) -> Trabajo {
        use flow_rust::descargador::paquete::{self, Opciones};
        use base64::Engine;
        let lote = &datos["lote"];
        let lote_dir = carpeta.join(lote["carpeta"].as_str().unwrap());
        for (ruta, b64) in lote["archivos"].as_object().unwrap() {
            let destino = lote_dir.join(ruta);
            std::fs::create_dir_all(destino.parent().unwrap()).unwrap();
            std::fs::write(destino, base64::engine::general_purpose::STANDARD.decode(b64.as_str().unwrap()).unwrap()).unwrap();
        }
        let zip = carpeta.join("captura.zip");
        paquete::exportar_con(&[lote_dir], &zip, &Opciones::default()).unwrap();
        let corte = NaiveDate::parse_from_str(datos["corte"].as_str().unwrap(), "%Y-%m-%d").unwrap();
        Trabajo::abrir_archivo(zip, Some(Mode::Cumplimiento), corte, &Config::default()).unwrap()
    }

    #[test]
    fn actualizar_desde_excel_en_un_paquete_igual_a_python() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/sync_paquete.json")).unwrap();
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let casos = datos["casos"].as_array().unwrap();
        let (mut iguales, mut fallos) = (0, Vec::new());
        for caso in casos {
            let carpeta = perfil.join("sync_paquete").join(caso["escenario"].as_str().unwrap());
            let _ = std::fs::remove_dir_all(&carpeta);
            std::fs::create_dir_all(&carpeta).unwrap();
            let d = comparar_con(trabajo_de_paquete(&datos, &carpeta), "CUMPLIMIENTO", caso);
            if d.is_empty() {
                iguales += 1;
            } else {
                fallos.push(format!("{}:\n  {}", caso["escenario"], d.join("\n  ")));
            }
        }
        for f in &fallos {
            println!("FALLA {f}");
        }
        println!("SYNC_PAQUETE: {iguales}/{}, fallos {}", casos.len(), fallos.len());
        assert_eq!(casos.len(), 6, "el oráculo debe traer 6 casos");
        assert!(fallos.is_empty());
    }

    /// Prueba permanente del oráculo: la regla que Python recalculó en el paquete, alterada, debe detectarse.
    #[test]
    fn oraculo_de_sync_en_paquete_detecta_alteraciones() {
        let datos: Value = serde_json::from_str(include_str!("../tests/datos/sync_paquete.json")).unwrap();
        let (_candado, perfil) = crate::prueba_entorno::perfil_aislado();
        let carpeta = perfil.join("sync_paquete_alterado");
        let _ = std::fs::remove_dir_all(&carpeta);
        std::fs::create_dir_all(&carpeta).unwrap();
        let mut caso = datos["casos"].as_array().unwrap().iter().find(|c| c["escenario"] == "origen_editado").unwrap().clone();
        assert!(comparar_con(trabajo_de_paquete(&datos, &carpeta), "CUMPLIMIENTO", &caso).is_empty());
        caso["reglas"][0] = serde_json::json!(["CUMPLIMIENTO.C05_POR_VENCER"]);
        let carpeta2 = perfil.join("sync_paquete_alterado_2");
        let _ = std::fs::remove_dir_all(&carpeta2);
        std::fs::create_dir_all(&carpeta2).unwrap();
        assert!(!comparar_con(trabajo_de_paquete(&datos, &carpeta2), "CUMPLIMIENTO", &caso).is_empty(), "una regla alterada no se detecta");
    }
}
