//! Transporte local de Flow para las extensiones Integral y Lite de Flow Python.
//! Escucha solo en 127.0.0.1. El código de conexión y las tareas viven en memoria; no se registra nada.

use serde_json::{json, Value};
use std::collections::{HashMap, VecDeque};
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};


/// Órdenes de lectura que admite la extensión Lite (`puente.Bridge.call` de Flow Python). Ninguna escribe en SITFA.
const COMANDOS: [&str; 11] = [
    "catalog",
    "capabilities",
    "lock",
    "unlock",
    "prepare",
    "search",
    "download",
    "search_and_download",
    "evidence_open",
    "evidence_capture",
    "evidence_close",
];
/// Lecturas de contexto (E-Q3): solo con la extensión Integral verificada en este puente (`verificar(Integral)`).
const LECTURAS_INTEGRAL: [&str; 4] = ["diary_open", "history_open", "calendar_open", "historico_open"];
const MOTIVOS_EVIDENCIA: [&str; 11] = [
    "estructura", "sesion", "cambio", "pantalla", "tiempo", "pagina", "acceso", "permiso", "pestaña", "ventana", "captura",
];
// El lector de la extensión admite 80 MiB binarios; base64 ocupa hasta 107 MiB.
const CONTROL_MAXIMO: usize = 256 * 1024;
const DATOS_MAXIMO: usize = 112 * 1024 * 1024;
const MEMORIA_CUERPOS_MAXIMO: usize = 160 * 1024 * 1024;
const CABECERAS_MAXIMO: usize = 16 * 1024;
const PLAZO_CABECERAS: Duration = Duration::from_secs(5);
const PLAZO_CUERPO: Duration = Duration::from_secs(30);
const PLAZO_ESCRITURA: Duration = Duration::from_secs(10);
const PLAZO_INACTIVO_KEEPALIVE: Duration = Duration::from_secs(10);
const MAX_PETICIONES_CONEXION: usize = 64;
const MAX_EN_VUELO: usize = 3;

fn tomar<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    // No se ejecuta código de usuario bajo estos locks; cada mutación es una operación de colección.
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn lectura_paralela(comando: &str) -> bool {
    LECTURAS_INTEGRAL.contains(&comando)
}

fn presupuesto(comando: &str) -> usize {
    if matches!(comando, "search" | "download" | "search_and_download" | "evidence_capture" | "evidence_open") || lectura_paralela(comando) {
        DATOS_MAXIMO
    } else {
        CONTROL_MAXIMO
    }
}

struct Pendiente {
    envio: Sender<Value>,
    comando: String,
}

#[derive(Default)]
struct Despacho {
    configurado: usize,
    fallas: usize,
    degradado: bool,
    en_vuelo: HashMap<String, (bool, Instant, Duration)>,
}

impl Despacho {
    fn limite(&self) -> usize {
        if self.degradado { 1 } else { self.configurado.clamp(1, MAX_EN_VUELO) }
    }
    fn purgar_vencidas(&mut self) {
        let ahora = Instant::now();
        self.en_vuelo.retain(|_, (_, inicio, plazo)| {
            ahora.saturating_duration_since(*inicio) <= *plazo + Duration::from_secs(90)
        });
    }
    fn admite(&self, comando: &str) -> bool {
        self.en_vuelo.is_empty() || (lectura_paralela(comando) && self.en_vuelo.values().all(|(p, _, _)| *p) && self.en_vuelo.len() < self.limite())
    }
    fn resultado(&mut self, sano: bool) {
        if sano { self.fallas = 0; } else {
            self.fallas = self.fallas.saturating_add(1);
            if self.fallas >= 2 { self.degradado = true; }
        }
    }
}

struct ReservaMemoria<'a> {
    contador: &'a AtomicUsize,
    reservado: usize,
}

impl<'a> ReservaMemoria<'a> {
    fn nueva(contador: &'a AtomicUsize) -> Self {
        Self { contador, reservado: 0 }
    }
    fn reservar(&mut self, adicional: usize) -> Result<(), ErrorHttp> {
        let mut actual = self.contador.load(Ordering::Relaxed);
        loop {
            let nuevo = actual.checked_add(adicional).ok_or(ErrorHttp::Grande)?;
            if nuevo > MEMORIA_CUERPOS_MAXIMO {
                return Err(ErrorHttp::Grande);
            }
            match self.contador.compare_exchange_weak(actual, nuevo, Ordering::AcqRel, Ordering::Relaxed) {
                Ok(_) => {
                    self.reservado += adicional;
                    return Ok(());
                }
                Err(v) => actual = v,
            }
        }
    }
}

impl Drop for ReservaMemoria<'_> {
    fn drop(&mut self) {
        if self.reservado > 0 {
            self.contador.fetch_sub(self.reservado, Ordering::AcqRel);
        }
    }
}

// Una conexión HTTP por socket. Plazo absoluto, no solo inactividad entre bytes.
struct LecturaHttp {
    socket: TcpStream,
    limite: Instant,
}
impl Read for LecturaHttp {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let resta = self.limite.saturating_duration_since(Instant::now());
        if resta.is_zero() { return Err(io::Error::new(io::ErrorKind::TimedOut, "plazo HTTP")); }
        self.socket.set_read_timeout(Some(resta))?;
        self.socket.read(buf)
    }
}

struct Peticion<'a> {
    entrada: &'a mut BufReader<LecturaHttp>,
    metodo: String,
    ruta: String,
    cabeceras: HashMap<String, String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ErrorHttp { Invalido, Grande, Tiempo }
impl From<io::Error> for ErrorHttp {
    fn from(e: io::Error) -> Self {
        if matches!(e.kind(), io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock) { Self::Tiempo } else { Self::Invalido }
    }
}
impl ErrorHttp {
    fn respuesta(self) -> (u16, Value) {
        match self {
            Self::Invalido => (400, json!({"error":"INVALID_MESSAGE", "message":"Mensaje HTTP inválido."})),
            Self::Grande => (413, json!({"error":"PAYLOAD_TOO_LARGE", "message":"La respuesta excede el presupuesto de lectura del puente local."})),
            Self::Tiempo => (408, json!({"error":"LOCAL_TIMEOUT", "message":"Se agotó el plazo de lectura del puente local."})),
        }
    }
}

fn linea<R: BufRead>(r: &mut R, restante: &mut usize) -> Result<String, ErrorHttp> {
    // read_until por sí solo no acota una línea sin salto; Take impone el máximo real.
    let mut b = Vec::new();
    let n = (&mut *r).take((*restante as u64).saturating_add(1)).read_until(b'\n', &mut b)?;
    if n > *restante { return Err(ErrorHttp::Grande); }
    *restante -= n;
    if !b.ends_with(b"\r\n") { return Err(ErrorHttp::Invalido); }
    b.truncate(b.len() - 2);
    String::from_utf8(b).map_err(|_| ErrorHttp::Invalido)
}

fn cabeceras_http_con_version<R: BufRead>(r: &mut R) -> Result<(String, String, String, HashMap<String, String>), ErrorHttp> {
    let mut resto = CABECERAS_MAXIMO;
    let primera = linea(r, &mut resto)?;
    let partes: Vec<_> = primera.split_whitespace().collect();
    if partes.len() != 3 || !matches!(partes[2], "HTTP/1.1" | "HTTP/1.0") || !partes[1].starts_with('/') { return Err(ErrorHttp::Invalido); }
    let mut headers = HashMap::new();
    loop {
        let l = linea(r, &mut resto)?;
        if l.is_empty() { break; }
        let (k, v) = l.split_once(':').ok_or(ErrorHttp::Invalido)?;
        if k.is_empty() || !k.bytes().all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)) || v.bytes().any(|b| b < 32 && b != b'\t' || b == 127) { return Err(ErrorHttp::Invalido); }
        let k = k.to_ascii_lowercase();
        if headers.insert(k.clone(), v.trim().to_string()).is_some() && matches!(k.as_str(), "content-length" | "transfer-encoding" | "origin" | "x-sitfa-code" | "x-sitfa-extension" | "x-sitfa-task") { return Err(ErrorHttp::Invalido); }
    }
    Ok((partes[0].into(), partes[1].into(), partes[2].into(), headers))
}

#[cfg(test)]
fn cabeceras_http<R: BufRead>(r: &mut R) -> Result<(String, String, HashMap<String, String>), ErrorHttp> {
    let (metodo, ruta, _, headers) = cabeceras_http_con_version(r)?;
    Ok((metodo, ruta, headers))
}

#[cfg(test)]
fn leer_cuerpo<R: BufRead>(r: &mut R, h: &HashMap<String, String>, max: usize) -> Result<Vec<u8>, ErrorHttp> {
    leer_cuerpo_con_reserva(r, h, max, None)
}

fn leer_cuerpo_con_reserva<R: BufRead>(
    r: &mut R,
    h: &HashMap<String, String>,
    max: usize,
    mut reserva: Option<&mut ReservaMemoria<'_>>,
) -> Result<Vec<u8>, ErrorHttp> {
    let largo = h.get("content-length");
    if let Some(te) = h.get("transfer-encoding") {
        if largo.is_some() || !te.eq_ignore_ascii_case("chunked") { return Err(ErrorHttp::Invalido); }
        let mut cuerpo = Vec::new();
        // También se limita el framing; permite muchos chunks pequeños sin crecimiento ilimitado.
        let mut framing = 8 * 1024 * 1024;
        loop {
            let l = linea(r, &mut framing)?;
            let n = l.split(';').next().ok_or(ErrorHttp::Invalido)?.trim();
            if n.is_empty() || !n.bytes().all(|b| b.is_ascii_hexdigit()) { return Err(ErrorHttp::Invalido); }
            let n = usize::from_str_radix(n, 16).map_err(|_| ErrorHttp::Grande)?;
            if n == 0 {
                let mut trailers = CABECERAS_MAXIMO;
                loop {
                    let l = linea(r, &mut trailers)?;
                    if l.is_empty() { return Ok(cuerpo); }
                    if !l.contains(':') { return Err(ErrorHttp::Invalido); }
                }
            }
            if n > max.saturating_sub(cuerpo.len()) { return Err(ErrorHttp::Grande); }
            if let Some(res) = reserva.as_deref_mut() { res.reservar(n)?; }
            let inicio = cuerpo.len();
            // No reservar el tamaño declarado antes de recibir bytes.
            let recibidos = (&mut *r).take(n as u64).read_to_end(&mut cuerpo)?;
            if recibidos != n || cuerpo.len() != inicio + n { return Err(ErrorHttp::Invalido); }
            let mut fin = [0; 2];
            r.read_exact(&mut fin)?;
            if fin != *b"\r\n" { return Err(ErrorHttp::Invalido); }
        }
    }
    let n = largo.ok_or(ErrorHttp::Invalido)?;
    if n.is_empty() || !n.bytes().all(|b| b.is_ascii_digit()) { return Err(ErrorHttp::Invalido); }
    let n = n.parse::<usize>().map_err(|_| ErrorHttp::Grande)?;
    if n == 0 { return Err(ErrorHttp::Invalido); }
    if n > max { return Err(ErrorHttp::Grande); }
    if let Some(res) = reserva { res.reservar(n)?; }
    let mut cuerpo = Vec::new();
    if (&mut *r).take(n as u64).read_to_end(&mut cuerpo)? != n { return Err(ErrorHttp::Invalido); }
    Ok(cuerpo)
}

fn respuesta_http(
    socket: &mut TcpStream,
    estado: u16,
    datos: Option<Value>,
    origen: Option<&str>,
    preflight: bool,
    mantener: bool,
) -> bool {
    let cuerpo = datos.map(|v| v.to_string()).unwrap_or_default();
    let razon = match estado { 200 => "OK", 204 => "No Content", 400 => "Bad Request", 403 => "Forbidden", 404 => "Not Found", 408 => "Request Timeout", 409 => "Conflict", 413 => "Payload Too Large", _ => "Error" };
    let _ = socket.set_write_timeout(Some(PLAZO_ESCRITURA));
    let escrito = (|| -> io::Result<()> {
        write!(socket, "HTTP/1.1 {estado} {razon}\r\n")?;
        if mantener {
            write!(socket, "Connection: keep-alive\r\nKeep-Alive: timeout={}, max={MAX_PETICIONES_CONEXION}\r\n", PLAZO_INACTIVO_KEEPALIVE.as_secs())?;
        } else {
            socket.write_all(b"Connection: close\r\n")?;
        }
        write!(socket, "Cache-Control: no-store\r\nVary: Origin\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\n", cuerpo.len())?;
        if let Some(o) = origen { write!(socket, "Access-Control-Allow-Origin: {o}\r\n")?; }
        if preflight {
            socket.write_all(b"Access-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type, X-Sitfa-Code, X-Sitfa-Extension, X-Sitfa-Task\r\nAccess-Control-Allow-Private-Network: true\r\n")?;
        }
        socket.write_all(b"\r\n")?;
        socket.write_all(cuerpo.as_bytes())?;
        socket.flush()
    })().is_ok();
    if !mantener || !escrito {
        let _ = socket.shutdown(std::net::Shutdown::Write);
    }
    mantener && escrito
}

fn aceptar(estado: &Estado, socket: TcpStream) {
    aceptar_con_plazo(estado, socket, PLAZO_INACTIVO_KEEPALIVE);
}

fn aceptar_con_plazo(estado: &Estado, socket: TcpStream, plazo_inactivo: Duration) {
    if socket.set_nonblocking(false).is_err() { return; }
    let mut entrada = BufReader::new(LecturaHttp { socket, limite: Instant::now() + PLAZO_CABECERAS });
    for numero in 1..=MAX_PETICIONES_CONEXION {
        if estado.cerrado.load(Ordering::SeqCst) {
            break;
        }
        if numero > 1 && entrada.buffer().is_empty() {
            entrada.get_mut().limite = Instant::now() + plazo_inactivo;
            match entrada.fill_buf() {
                Ok([]) => break,
                Ok(_) => {}
                Err(_) => break,
            }
        }
        entrada.get_mut().limite = Instant::now() + PLAZO_CABECERAS;
        let (metodo, ruta, version, cabeceras) = match cabeceras_http_con_version(&mut entrada) {
            Ok(v) => v,
            Err(e) => {
                let (status, datos) = e.respuesta();
                respuesta_http(&mut entrada.get_mut().socket, status, Some(datos), None, false, false);
                break;
            }
        };
        let conn = cabeceras.get("connection").map(String::as_str).unwrap_or("");
        let pide_cerrar = conn.split(',').any(|t| t.trim().eq_ignore_ascii_case("close"));
        let pide_keep = conn.split(',').any(|t| t.trim().eq_ignore_ascii_case("keep-alive"));
        let quiere_keep = if pide_cerrar {
            false
        } else if version == "HTTP/1.0" {
            pide_keep
        } else {
            true
        };
        let mantener = quiere_keep && numero < MAX_PETICIONES_CONEXION && !estado.cerrado.load(Ordering::SeqCst);
        let mut peticion = Peticion { entrada: &mut entrada, metodo, ruta, cabeceras };
        if !atender(estado, &mut peticion, mantener) {
            break;
        }
    }
}
const ESPERA_TAREA: Duration = Duration::from_secs(4);
const VIGENCIA_CONEXION: Duration = Duration::from_secs(20);

struct Tarea {
    id: String,
    comando: String,
    payload: Value,
    plazo: Duration,
}

struct Estado {
    puerto: u16,
    codigo: String,
    origen: Mutex<Option<String>>,
    tareas: Mutex<VecDeque<Tarea>>,
    hay_tarea: Condvar,
    pendientes: Mutex<HashMap<String, Pendiente>>,
    despacho: Mutex<Despacho>,
    visto: Mutex<Option<Instant>>,
    memoria_cuerpos: AtomicUsize,
    cerrado: AtomicBool,
    /// La última verificación de la extensión fue Integral: habilita `LECTURAS_INTEGRAL`.
    integral: AtomicBool,
    /// La extensión Integral conectada anuncia soporte `flow_rust_rapido` (2.10.0 / 2.11.0).
    rapido_ext: AtomicBool,
    /// La extensión Integral conectada anuncia `search_and_download` (2.11.0+).
    search_and_download: AtomicBool,
}

pub struct Puente {
    puerto: u16,
    estado: Arc<Estado>,
}

fn aleatorio<const N: usize>() -> Result<String, String> {
    let mut datos = [0u8; N];
    getrandom::fill(&mut datos).map_err(|_| "No se pudo generar el código local.")?;
    Ok(datos.iter().map(|b| format!("{b:02x}")).collect())
}

fn es_id_extension(texto: &str) -> bool {
    texto.len() == 32 && texto.bytes().all(|b| (b'a'..=b'p').contains(&b))
}

fn iguales_en_tiempo_constante(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

fn cabecera(peticion: &Peticion, nombre: &str) -> Option<String> {
    peticion.cabeceras.get(&nombre.to_ascii_lowercase()).cloned()
}

/// Origen CORS válido: `chrome-extension://<id>`.
fn origen_cors(peticion: &Peticion) -> Option<String> {
    cabecera(peticion, "Origin").filter(|o| {
        o.strip_prefix("chrome-extension://")
            .is_some_and(es_id_extension)
    })
}

/// Identidad de la extensión. Chrome puede omitir `Origin`; la identidad declarada nunca reemplaza al código secreto.
fn origen_extension(peticion: &Peticion) -> Option<String> {
    let declarada = cabecera(peticion, "X-Sitfa-Extension").unwrap_or_default();
    if !declarada.is_empty() && !es_id_extension(&declarada) {
        return None;
    }
    match cabecera(peticion, "Origin") {
        Some(origen) => {
            let valido = origen_cors(peticion)?;
            if !declarada.is_empty() && origen != format!("chrome-extension://{declarada}") {
                return None;
            }
            Some(valido)
        }
        None => (!declarada.is_empty()).then(|| format!("chrome-extension://{declarada}")),
    }
}

fn responder(peticion: &mut Peticion<'_>, estado_http: u16, datos: Value, mantener: bool) -> bool {
    let origen = origen_cors(peticion);
    respuesta_http(&mut peticion.entrada.get_mut().socket, estado_http, Some(datos), origen.as_deref(), false, mantener)
}

fn autorizada(estado: &Estado, peticion: &Peticion) -> Result<String, &'static str> {
    let origen = origen_extension(peticion).ok_or("EXTENSION_ORIGIN")?;
    let codigo = cabecera(peticion, "X-Sitfa-Code").unwrap_or_default();
    let formato = codigo.len() == 32
        && codigo
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !formato || !iguales_en_tiempo_constante(&codigo, &estado.codigo) {
        return Err("CONNECTION_CODE");
    }
    if tomar(&estado.origen)
        .as_ref()
        .is_some_and(|fijado| *fijado != origen)
    {
        return Err("EXTENSION_CHANGED");
    }
    *tomar(&estado.visto) = Some(Instant::now());
    Ok(origen)
}

fn atender(estado: &Estado, peticion: &mut Peticion<'_>, mantener: bool) -> bool {
    let host_valido = cabecera(peticion, "Host").is_some_and(|h| {
        h.eq_ignore_ascii_case(&format!("127.0.0.1:{}", estado.puerto))
            || h.eq_ignore_ascii_case(&format!("localhost:{}", estado.puerto))
    });
    if !host_valido {
        return responder(peticion, 403, json!({"error": "INVALID_HOST"}), false);
    }
    if peticion.metodo == "OPTIONS" {
        let Some(origen) = origen_cors(peticion) else { return responder(peticion, 403, json!({}), false); };
        if tomar(&estado.origen).as_ref().is_some_and(|fijado| *fijado != origen) {
            return responder(peticion, 403, json!({"error": "EXTENSION_CHANGED"}), false);
        }
        return respuesta_http(&mut peticion.entrada.get_mut().socket, 204, None, Some(&origen), true, mantener);
    }
    let origen = match autorizada(estado, peticion) {
        Ok(o) => o,
        Err(motivo) => return responder(peticion, 403, json!({"error": motivo}), false),
    };
    let ruta = peticion.ruta.clone();
    if peticion.metodo == "GET" {
        if peticion.cabeceras.contains_key("content-length") || peticion.cabeceras.contains_key("transfer-encoding") {
            return responder(peticion, 400, json!({"error": "INVALID_MESSAGE"}), false);
        }
        if ruta != "/next" || tomar(&estado.origen).is_none() { return responder(peticion, 404, json!({}), mantener); }
        let limite = Instant::now() + ESPERA_TAREA;
        let mut cola = tomar(&estado.tareas);
        loop {
            while let Some(tarea) = cola.front() {
                let pendientes = tomar(&estado.pendientes);
                if !pendientes.contains_key(&tarea.id) { cola.pop_front(); continue; }
                let mut d = tomar(&estado.despacho);
                d.purgar_vencidas();
                if !d.admite(&tarea.comando) || (!estado.integral.load(Ordering::SeqCst) && !d.en_vuelo.is_empty()) { break; }
                let Some(tarea) = cola.pop_front() else { break; };
                d.en_vuelo.insert(tarea.id.clone(), (lectura_paralela(&tarea.comando), Instant::now(), tarea.plazo));
                let paralelo = if estado.integral.load(Ordering::SeqCst) { d.limite() } else { 1 };
                drop(d); drop(pendientes); drop(cola);
                return responder(peticion, 200, json!({"id":tarea.id,"command":tarea.comando,"payload":tarea.payload,"parallel":paralelo}), mantener);
            }
            let resta = limite.saturating_duration_since(Instant::now());
            if resta.is_zero() || estado.cerrado.load(Ordering::SeqCst) {
                drop(cola);
                let paralelo = if estado.integral.load(Ordering::SeqCst) { tomar(&estado.despacho).limite() } else { 1 };
                return responder(peticion, 200, json!({"idle":true,"parallel":paralelo}), mantener);
            }
            cola = estado.hay_tarea.wait_timeout(cola, resta).unwrap_or_else(|e| e.into_inner()).0;
        }
    }
    if peticion.metodo != "POST" || !matches!(ruta.as_str(), "/connect" | "/complete") {
        return responder(peticion, 404, json!({}), false);
    }
    let tarea_declarada = cabecera(peticion, "X-Sitfa-Task");
    let maximo = if ruta == "/connect" { CONTROL_MAXIMO } else if let Some(id) = &tarea_declarada {
        let pendientes = tomar(&estado.pendientes);
        // Una respuesta tardía también libera el cupo sin interpretar un cuerpo arbitrario.
        match pendientes.get(id) {
            Some(p) => presupuesto(&p.comando),
            None => {
                drop(pendientes);
                tomar(&estado.despacho).en_vuelo.remove(id);
                estado.hay_tarea.notify_all();
                return responder(peticion, 409, json!({"error":"TASK_EXPIRED"}), false);
            }
        }
    } else {
        // Extensión anterior: si hay tareas activas identificadas, también se aplica su presupuesto
        // antes de leer. Una respuesta tardía sin cabecera necesita el límite general para liberar el cupo.
        let pendientes = tomar(&estado.pendientes);
        let d = tomar(&estado.despacho);
        d.en_vuelo.keys().map(|id| pendientes.get(id).map_or(DATOS_MAXIMO, |p| presupuesto(&p.comando))).max().unwrap_or(CONTROL_MAXIMO)
    };
    peticion.entrada.get_mut().limite = Instant::now() + PLAZO_CUERPO;
    if cabecera(peticion, "Expect").is_some_and(|v| v.eq_ignore_ascii_case("100-continue")) {
        let _ = peticion.entrada.get_mut().socket.set_write_timeout(Some(PLAZO_ESCRITURA));
        let _ = peticion.entrada.get_mut().socket.write_all(b"HTTP/1.1 100 Continue\r\n\r\n");
    }
    let mut reserva = ReservaMemoria::nueva(&estado.memoria_cuerpos);
    let cuerpo = match leer_cuerpo_con_reserva(peticion.entrada, &peticion.cabeceras, maximo, Some(&mut reserva)) {
        Ok(c) => c,
        Err(e) => { let (status, datos) = e.respuesta(); return responder(peticion, status, datos, false); }
    };
    let Ok(Value::Object(datos)) = serde_json::from_slice::<Value>(&cuerpo) else {
        return responder(peticion, 400, json!({"error":"INVALID_MESSAGE"}), mantener);
    };
    if ruta == "/connect" {
        let mut fijado = tomar(&estado.origen);
        if fijado.as_ref().is_some_and(|fijado| *fijado != origen) {
            return responder(peticion, 403, json!({"error":"EXTENSION_CHANGED"}), mantener);
        }
        *fijado = Some(origen);
        *tomar(&estado.visto) = Some(Instant::now());
        drop(fijado);
        tomar(&estado.despacho).en_vuelo.clear();
        estado.hay_tarea.notify_all();
        return responder(peticion, 200, json!({"ok":true,"parallel":1}), mantener);
    }
    if tomar(&estado.origen).is_none() { return responder(peticion, 409, json!({"error":"NOT_CONNECTED"}), mantener); }
    let (Some(id), Some(sano)) = (datos.get("id").and_then(Value::as_str), datos.get("ok").and_then(Value::as_bool)) else {
        return responder(peticion, 400, json!({"error":"INVALID_MESSAGE"}), mantener);
    };
    if tarea_declarada.as_deref().is_some_and(|t| t != id) {
        return responder(peticion, 400, json!({"error":"INVALID_MESSAGE"}), mantener);
    }
    let mut pendientes = tomar(&estado.pendientes);
    if pendientes.get(id).is_some_and(|p| cuerpo.len() > presupuesto(&p.comando)) {
        pendientes.remove(id);
        tomar(&estado.despacho).en_vuelo.remove(id);
        drop(pendientes);
        estado.hay_tarea.notify_all();
        let (status, datos) = ErrorHttp::Grande.respuesta();
        return responder(peticion, status, datos, mantener);
    }
    let mut d = tomar(&estado.despacho);
    let activa = d.en_vuelo.remove(id).is_some();
    if activa { d.resultado(sano); }
    drop(d);
    let canal = if activa { pendientes.remove(id) } else { None };
    drop(pendientes);
    estado.hay_tarea.notify_all();
    if canal.is_some_and(|p| p.envio.send(Value::Object(datos)).is_ok()) {
        responder(peticion, 200, json!({"ok":true}), mantener)
    } else {
        responder(peticion, 409, json!({"error":"TASK_EXPIRED"}), mantener)
    }
}

/// Texto fijo por código de la extensión; el motivo técnico de la evidencia solo se acepta de una lista cerrada.
fn mensaje_error(comando: &str, datos: &Value) -> String {
    let codigo = datos["error"].as_str().unwrap_or("");
    let mut texto = match codigo {
        "PAYLOAD_TOO_LARGE" => "La respuesta excede el presupuesto de lectura del puente local; acota la consulta.",
        "LOCAL_TIMEOUT" => "Se agotó el plazo de lectura del puente local.",
        "FOLLOWUP_REQUIRED" => "Entra a Seguimiento en la pestaña SITFA y vuelve a cargar las opciones.",
        "SITE_ACCESS" => "Comprueba que Chrome siga abierto en SITFA y que la extensión pueda acceder a esa pestaña.",
        "MULTIPLE_FORMS" => "Se encontró más de un formulario de Seguimiento. Abre la conexión desde una pestaña con un solo formulario.",
        "SCREEN_OPEN" => "No se pudo abrir la pantalla elegida. Revisa la sesión SITFA y vuelve a cargar las opciones.",
        "EVIDENCE_CAPTURE" => "No se pudo capturar la consulta vacía. Mantén SITFA visible, pulsa el icono de la extensión desde SITFA y vuelve a conectar. Los archivos guardados se conservan.",
        "EVIDENCE_OPEN" => "No se pudo cargar la vista SITFA para generar el PDF de la consulta vacía. Revisa la sesión y actualiza la extensión. Los archivos guardados se conservan.",
        _ if comando == "catalog" => "La extensión no pudo cargar el catálogo. Revisa la sesión de SITFA y la pestaña de conexión.",
        _ => "La extensión no pudo completar la consulta. Revisa su pestaña de conexión y la sesión SITFA.",
    }
    .to_string();
    if let Some(motivo) = datos["motivo"]
        .as_str()
        .filter(|m| matches!(codigo, "EVIDENCE_CAPTURE" | "EVIDENCE_OPEN") && MOTIVOS_EVIDENCIA.contains(m))
    {
        texto.push_str(&format!(" Motivo técnico: {motivo}."));
    }
    texto
}

impl Puente {
    /// Abre el puente en un puerto libre de 127.0.0.1.
    pub fn abrir() -> Result<Puente, String> {
        let servidor = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("No se pudo abrir el puente local: {e}"))?;
        servidor.set_nonblocking(true).map_err(|e| e.to_string())?;
        let puerto = servidor.local_addr().map_err(|e| e.to_string())?.port();
        let estado = Arc::new(Estado {
            puerto,
            codigo: aleatorio::<16>()?,
            origen: Mutex::new(None),
            tareas: Mutex::new(VecDeque::new()),
            hay_tarea: Condvar::new(),
            pendientes: Mutex::new(HashMap::new()),
            despacho: Mutex::new(Despacho::default()),
            visto: Mutex::new(None),
            memoria_cuerpos: AtomicUsize::new(0),
            cerrado: AtomicBool::new(false),
            integral: AtomicBool::new(false),
            rapido_ext: AtomicBool::new(false),
            search_and_download: AtomicBool::new(false),
        });
        // Tres peticiones de trabajo como máximo, más polls y control; número fijo de hilos.
        let servidor = Arc::new(servidor);
        for _ in 0..8 {
            let (servidor, estado) = (servidor.clone(), estado.clone());
            std::thread::spawn(move || {
                while !estado.cerrado.load(Ordering::SeqCst) {
                    match servidor.accept() {
                        Ok((socket, _)) => aceptar(&estado, socket),
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(50)),
                        Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                        Err(_) => break,
                    }
                }
            });
        }
        Ok(Puente { puerto, estado })
    }

    /// Configura el tope para la próxima operación. Una degradación por fallas dura hasta reconectar.
    pub fn configurar_concurrencia(&self, maximo: usize) {
        let nuevo = maximo.clamp(1, MAX_EN_VUELO);
        let mut d = tomar(&self.estado.despacho);
        if d.configurado != nuevo {
            d.configurado = nuevo;
            drop(d);
            self.estado.hay_tarea.notify_all();
        }
    }

    /// La extensión verificada es Integral (habilita las lecturas de contexto) o no (Lite, o verificación fallida).
    pub(super) fn habilitar_lecturas(&self, integral: bool) {
        self.estado.integral.store(integral, Ordering::SeqCst);
    }

    pub(super) fn registrar_rapido(&self, soportado: bool) {
        self.estado.rapido_ext.store(soportado, Ordering::SeqCst);
    }

    /// La extensión Integral conectada es 2.10.0+ con soporte para Modo Rápido Seguro.
    pub fn soporta_rapido(&self) -> bool {
        self.estado.rapido_ext.load(Ordering::SeqCst)
    }

    #[allow(dead_code)]
    pub(super) fn registrar_search_and_download(&self, soportado: bool) {
        self.estado.search_and_download.store(soportado, Ordering::SeqCst);
    }

    /// La extensión Integral conectada es 2.11.0+ con soporte para `search_and_download`.
    pub fn soporta_search_and_download(&self) -> bool {
        self.estado.search_and_download.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    fn puerto(&self) -> u16 {
        self.puerto
    }

    /// Lo que el usuario pega en la extensión: `<puerto>-<32 hex>`. Cambia cada vez que se abre el puente.
    pub fn codigo_conexion(&self) -> String {
        format!("{}-{}", self.puerto, self.estado.codigo)
    }

    /// La extensión se conectó y sigue consultando.
    pub fn conectado(&self) -> bool {
        !self.estado.cerrado.load(Ordering::SeqCst)
            && tomar(&self.estado.origen).is_some()
            && tomar(&self.estado.visto)
                .is_some_and(|t| t.elapsed() < VIGENCIA_CONEXION)
    }

    pub(super) fn llamar(
        &self,
        comando: &str,
        payload: Value,
        plazo: Duration,
    ) -> Result<Value, String> {
        if comando == "context_batch" {
            // Orden interna del adaptador, no una acción del navegador. Solo ventanas de lectura verificadas.
            let comando = payload["command"].as_str().filter(|c| lectura_paralela(c)).ok_or("Acción no permitida.")?;
            let cargas = payload["payloads"].as_array().filter(|a| !a.is_empty() && a.len() <= MAX_EN_VUELO).ok_or("Grupo de lectura inválido.")?;
            if !self.estado.integral.load(Ordering::SeqCst) { return Err("Acción no permitida.".into()); }
            let mut resultados = Vec::with_capacity(cargas.len());
            let mut restantes = cargas.as_slice();
            while !restantes.is_empty() {
                let n = restantes.len().min(tomar(&self.estado.despacho).limite());
                let grupo = &restantes[..n];
                // Se conservan las posiciones aunque Chrome responda fuera de orden.
                let r = std::thread::scope(|scope| {
                    let h: Vec<_> = grupo.iter().map(|p| scope.spawn(move || self.llamar(comando, p.clone(), plazo))).collect();
                    h.into_iter().map(|t| match t.join() {
                        Ok(Ok(v)) => json!({"ok":true,"result":v}),
                        Ok(Err(e)) => json!({"ok":false,"error":e}),
                        Err(_) => json!({"ok":false,"error":"Falló el trabajador de lectura local."}),
                    }).collect::<Vec<_>>()
                });
                resultados.extend(r);
                restantes = &restantes[n..];
            }
            return Ok(Value::Array(resultados));
        }
        let lectura = LECTURAS_INTEGRAL.contains(&comando);
        if !COMANDOS.contains(&comando) && !(lectura && self.estado.integral.load(Ordering::SeqCst)) {
            return Err("Acción no permitida.".into());
        }
        if !self.conectado() {
            return Err("Conecta la extensión desde la pestaña SITFA y mantén abierta su ventana de conexión.".into());
        }
        let id = aleatorio::<12>()?;
        let (envio, recepcion) = channel();
        {
            let mut pendientes = tomar(&self.estado.pendientes);
            if self.estado.cerrado.load(Ordering::SeqCst) {
                return Err("Se cerró la conexión local.".into());
            }
            pendientes.insert(id.clone(), Pendiente { envio, comando: comando.into() });
        }
        tomar(&self.estado.tareas).push_back(Tarea {
            id: id.clone(),
            comando: comando.into(),
            payload,
            plazo,
        });
        self.estado.hay_tarea.notify_all();
        let respuesta = recepcion.recv_timeout(plazo);
        tomar(&self.estado.pendientes).remove(&id);
        if respuesta.is_err() {
            // No liberar un fetch en vuelo hasta que Chrome lo confirme, para no exceder el tope real.
            tomar(&self.estado.despacho).resultado(false);
        }
        self.estado.hay_tarea.notify_all();
        let datos = respuesta.map_err(|_| {
            "Chrome no respondió a tiempo o se cerró la conexión local. Los archivos ya verificados se conservan."
        })?;
        if datos["ok"].as_bool() == Some(true) {
            return Ok(datos["result"].clone());
        }
        Err(mensaje_error(comando, &datos))
    }

    pub fn cerrar(&self) {
        self.estado.cerrado.store(true, Ordering::SeqCst);
        tomar(&self.estado.pendientes).clear();
        *tomar(&self.estado.origen) = None;
        self.estado.hay_tarea.notify_all();
        tomar(&self.estado.tareas).clear();
        tomar(&self.estado.despacho).en_vuelo.clear();
    }
}

impl Drop for Puente {
    fn drop(&mut self) {
        self.cerrar();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descargador::{Edicion, Pantalla};
    use std::io::Write;
    use std::net::{Ipv4Addr, TcpStream};

    fn pedir(
        puente: &Puente,
        method: &str,
        ruta: &str,
        origen: &str,
        cuerpo: &Value,
    ) -> (u16, Value) {
        let mut socket = TcpStream::connect((Ipv4Addr::LOCALHOST, puente.puerto())).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(8)))
            .unwrap();
        let datos = if method == "POST" { cuerpo.to_string() } else { String::new() };
        if method == "POST" {
            write!(
                socket,
                "{method} {ruta} HTTP/1.1\r\nHost: localhost:{}\r\nOrigin: {origen}\r\nX-Sitfa-Code: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{datos}",
                puente.puerto(),
                puente.estado.codigo,
                datos.len(),
            ).unwrap();
        } else {
            write!(
                socket,
                "{method} {ruta} HTTP/1.1\r\nHost: localhost:{}\r\nOrigin: {origen}\r\nX-Sitfa-Code: {}\r\nConnection: close\r\n\r\n",
                puente.puerto(),
                puente.estado.codigo,
            ).unwrap();
        }
        let mut respuesta = String::new();
        socket.read_to_string(&mut respuesta).unwrap();
        let (cabecera, cuerpo) = respuesta.split_once("\r\n\r\n").unwrap();
        let status = cabecera.split_whitespace().nth(1).unwrap().parse().unwrap();
        (status, serde_json::from_str(cuerpo).unwrap())
    }

    #[test]
    fn chunked_valido_y_presupuesto_real() {
        let puente = Puente::abrir().unwrap();
        let mut socket = TcpStream::connect((Ipv4Addr::LOCALHOST, puente.puerto())).unwrap();
        socket.set_read_timeout(Some(Duration::from_secs(8))).unwrap();
        write!(socket, "POST /connect HTTP/1.1\r\nHost: localhost:{}\r\nOrigin: chrome-extension://{}\r\nX-Sitfa-Code: {}\r\nConnection: close\r\nTransfer-Encoding: chunked\r\n\r\n1\r\n{{\r\n1\r\n}}\r\n0\r\n\r\n", puente.puerto(), "a".repeat(32), puente.estado.codigo).unwrap();
        let mut salida = String::new(); socket.read_to_string(&mut salida).unwrap();
        assert!(salida.starts_with("HTTP/1.1 200"));
        let h = HashMap::from([("transfer-encoding".to_string(), "chunked".to_string())]);
        let mut b = std::io::Cursor::new(b"2\r\n{}\r\n0\r\nX-Test: valor\r\n\r\n");
        assert_eq!(leer_cuerpo(&mut b, &h, 2).unwrap(), b"{}");
        let mut exceso = std::io::Cursor::new(b"2\r\n{}\r\n1\r\nx\r\n0\r\n\r\n");
        let e = leer_cuerpo(&mut exceso, &h, 2).unwrap_err();
        assert_eq!(e, ErrorHttp::Grande);
        assert_eq!(e.respuesta().0, 413);
        assert_eq!(e.respuesta().1["error"], "PAYLOAD_TOO_LARGE");
        assert!(e.respuesta().1["message"].as_str().unwrap().contains("presupuesto"));
        let h = HashMap::from([("content-length".to_string(), (CONTROL_MAXIMO + 1).to_string())]);
        assert_eq!(leer_cuerpo(&mut std::io::Cursor::new(b""), &h, CONTROL_MAXIMO), Err(ErrorHttp::Grande));
        assert!(presupuesto("download") > presupuesto("catalog"));
    }

    #[test]
    fn plazo_absoluto_y_mutex_envenenado() {
        let servidor = TcpListener::bind("127.0.0.1:0").unwrap();
        let _cliente = TcpStream::connect(servidor.local_addr().unwrap()).unwrap();
        let (socket, _) = servidor.accept().unwrap();
        let mut r = BufReader::new(LecturaHttp { socket, limite: Instant::now() + Duration::from_millis(50) });
        assert_eq!(cabeceras_http(&mut r).unwrap_err(), ErrorHttp::Tiempo);
        let m = Arc::new(Mutex::new(7));
        let copia = m.clone();
        assert!(std::thread::spawn(move || { let _g = tomar(&copia); panic!("fallo sintético"); }).join().is_err());
        assert_eq!(*tomar(&m), 7);
    }

    #[test]
    fn despacho_tope_tres_y_degradacion() {
        let mut d = Despacho { configurado: 99, ..Default::default() };
        for i in 0..3 {
            assert!(d.admite("diary_open"));
            d.en_vuelo.insert(i.to_string(), (true, Instant::now(), Duration::from_secs(30)));
        }
        assert!(!d.admite("diary_open")); assert!(!d.admite("search"));
        d.en_vuelo.clear(); d.resultado(false); assert_eq!(d.limite(), 3);
        d.resultado(false); assert_eq!(d.limite(), 1);
        d.en_vuelo.insert("sintetica".into(), (true, Instant::now(), Duration::from_secs(30)));
        assert!(!d.admite("history_open"));
        d.resultado(true); assert_eq!(d.limite(), 1);
    }

    #[test]
    fn grupo_real_entrega_tres_y_reordena_respuestas() {
        let p = Puente::abrir().unwrap();
        let origen = format!("chrome-extension://{}", "a".repeat(32));
        let vacio = json!({});
        assert_eq!(pedir(&p, "POST", "/connect", &origen, &vacio).0, 200);
        p.habilitar_lecturas(true);
        p.configurar_concurrencia(3);
        std::thread::scope(|scope| {
            let lectura = scope.spawn(|| p.llamar("context_batch", json!({"command":"diary_open","payloads":[{"i":0},{"i":1},{"i":2}]}), Duration::from_secs(10)));
            let mut tareas = Vec::new();
            for _ in 0..3 {
                let (status, t) = pedir(&p, "GET", "/next", &origen, &vacio);
                assert_eq!(status, 200); assert_eq!(t["command"], "diary_open");
                tareas.push(t);
            }
            assert_eq!(tomar(&p.estado.despacho).en_vuelo.len(), 3);
            for t in tareas.into_iter().rev() {
                let respuesta = json!({"id":t["id"],"ok":true,"result":t["payload"]});
                assert_eq!(pedir(&p, "POST", "/complete", &origen, &respuesta).0, 200);
            }
            let v = lectura.join().unwrap().unwrap();
            assert_eq!(v, json!([{"ok":true,"result":{"i":0}},{"ok":true,"result":{"i":1}},{"ok":true,"result":{"i":2}}]));
            assert!(tomar(&p.estado.despacho).en_vuelo.is_empty());
        });
    }

    #[test]
    fn catalogo_lite_por_el_puente_local_con_extension_ficticia() {
        let puente = Puente::abrir().unwrap();
        let extension = format!("chrome-extension://{}", "a".repeat(32));
        let otra = format!("chrome-extension://{}", "b".repeat(32));
        let vacio = json!({});
        assert_eq!(
            pedir(
                &puente,
                "POST",
                "/connect",
                "https://sitio-ficticio.invalid",
                &vacio
            )
            .0,
            403
        );
        assert_eq!(
            pedir(&puente, "POST", "/connect", &extension, &vacio).0,
            200
        );
        assert_eq!(pedir(&puente, "POST", "/connect", &otra, &vacio).0, 403);
        // E-Q3: sin la verificación Integral (Lite o recién conectado) las lecturas de contexto se rechazan.
        for excluida in ["diary_open", "history_open", "calendar_open", "navigate", "carga"] {
            assert_eq!(
                puente.llamar(excluida, vacio.clone(), Duration::ZERO).unwrap_err(),
                "Acción no permitida."
            );
        }
        puente.habilitar_lecturas(true);
        for lectura in ["diary_open", "history_open", "calendar_open"] {
            assert_ne!(
                puente.llamar(lectura, vacio.clone(), Duration::ZERO).unwrap_err(),
                "Acción no permitida.",
                "{lectura} debe pasar el filtro con Integral verificada"
            );
        }
        for excluida in ["navigate", "carga"] {
            assert_eq!(puente.llamar(excluida, vacio.clone(), Duration::ZERO).unwrap_err(), "Acción no permitida.");
        }
        puente.habilitar_lecturas(false);
        assert_eq!(puente.llamar("diary_open", vacio.clone(), Duration::ZERO).unwrap_err(), "Acción no permitida.");

        std::thread::scope(|scope| {
            let trabajo = scope.spawn(|| puente.catalogo(Edicion::Lite, Pantalla::Litigantes));
            for (orden, result) in [
                (
                    "catalog",
                    json!({
                        "pantalla": "litigantes", "tribunales": {"901": "Tribunal ficticio"},
                        "estados": {"1": "Pendientes"}
                    }),
                ),
                (
                    "capabilities",
                    json!({
                        "tipo": "SITFA_LITE_CAPACIDADES", "version": "1.1.0", "escritura_rus": false,
                        "protocolos": {"descarga_xls": 1, "pdf_sin_registros": 1}
                    }),
                ),
            ] {
                let (status, tarea) = pedir(&puente, "GET", "/next", &extension, &vacio);
                assert_eq!(status, 200);
                assert_eq!(tarea["command"], orden);
                if orden == "catalog" {
                    assert_eq!(tarea["payload"]["screen"], "litigantes");
                }
                let mensaje = json!({"id": tarea["id"], "ok": true, "result": result});
                assert_eq!(
                    pedir(&puente, "POST", "/complete", &extension, &mensaje).0,
                    200
                );
            }
            let catalogo = trabajo.join().unwrap().unwrap();
            assert_eq!(catalogo.pantalla, Pantalla::Litigantes);
            assert_eq!(catalogo.tribunales["901"], "Tribunal ficticio");
        });
    }

    fn leer_respuesta_keep_alive(r: &mut BufReader<TcpStream>) -> io::Result<(u16, HashMap<String, String>, Value)> {
        let mut linea_estado = String::new();
        if r.read_line(&mut linea_estado)? == 0 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "eof"));
        }
        let status: u16 = linea_estado
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "status"))?;
        let mut headers = HashMap::new();
        loop {
            let mut l = String::new();
            r.read_line(&mut l)?;
            let trimmed = l.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                break;
            }
            let (k, v) = trimmed.split_once(':').unwrap();
            headers.insert(k.to_ascii_lowercase(), v.trim().to_string());
        }
        let len: usize = headers
            .get("content-length")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let mut body = vec![0u8; len];
        r.read_exact(&mut body)?;
        let json = if body.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&body).unwrap()
        };
        Ok((status, headers, json))
    }

    #[test]
    fn puente_soporta_keep_alive_acotado() {
        let puente = Puente::abrir().unwrap();
        assert!(COMANDOS.contains(&"search_and_download"));
        assert!(presupuesto("search_and_download") > presupuesto("catalog"));
        assert!(!puente.soporta_search_and_download());
        puente.registrar_search_and_download(true);
        assert!(puente.soporta_search_and_download());

        let origen = format!("chrome-extension://{}", "a".repeat(32));

        // 1) Un solo socket TCP reutiliza 64 peticiones consecutivas; la 64ª responde `Connection: close` y cierra.
        let mut sock = TcpStream::connect((Ipv4Addr::LOCALHOST, puente.puerto())).unwrap();
        sock.set_read_timeout(Some(Duration::from_secs(8))).unwrap();
        let mut lector = BufReader::new(sock.try_clone().unwrap());
        for i in 1..=MAX_PETICIONES_CONEXION {
            let body = "{}";
            write!(
                sock,
                "POST /connect HTTP/1.1\r\nHost: localhost:{}\r\nOrigin: {origen}\r\nX-Sitfa-Code: {}\r\nContent-Length: {}\r\n\r\n{body}",
                puente.puerto(),
                puente.estado.codigo,
                body.len()
            )
            .unwrap();
            let (status, headers, datos) = leer_respuesta_keep_alive(&mut lector).unwrap();
            assert_eq!(status, 200, "iteración {i}");
            assert_eq!(datos["ok"], true);
            if i < MAX_PETICIONES_CONEXION {
                assert_eq!(headers.get("connection").map(String::as_str), Some("keep-alive"), "iteración {i}");
                assert!(headers.get("keep-alive").is_some_and(|v| v.contains("max=64")));
            } else {
                assert_eq!(headers.get("connection").map(String::as_str), Some("close"), "iteración 64 debe cerrar");
            }
        }
        // Después de la petición 64, el servidor cerró el socket (EOF inmediato).
        assert!(leer_respuesta_keep_alive(&mut lector).is_err());

        // 2) `HTTP/1.0` sin `Connection: keep-alive` cierra tras 1 petición.
        let mut s10 = TcpStream::connect((Ipv4Addr::LOCALHOST, puente.puerto())).unwrap();
        s10.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        write!(
            s10,
            "POST /connect HTTP/1.0\r\nHost: localhost:{}\r\nOrigin: {origen}\r\nX-Sitfa-Code: {}\r\nContent-Length: 2\r\n\r\n{{}}",
            puente.puerto(),
            puente.estado.codigo,
        )
        .unwrap();
        let mut r10 = BufReader::new(s10);
        let (st10, hdr10, _) = leer_respuesta_keep_alive(&mut r10).unwrap();
        assert_eq!(st10, 200);
        assert_eq!(hdr10.get("connection").map(String::as_str), Some("close"));
        assert!(leer_respuesta_keep_alive(&mut r10).is_err());

        // 3) Timeout de inactividad entre peticiones cierra limpiamente la conexión sin error.
        let servidor = TcpListener::bind("127.0.0.1:0").unwrap();
        let puerto_t = servidor.local_addr().unwrap().port();
        let estado_t = puente.estado.clone();
        let hilo = std::thread::spawn(move || {
            let (sock_srv, _) = servidor.accept().unwrap();
            aceptar_con_plazo(&estado_t, sock_srv, Duration::from_millis(80));
        });
        let mut cli = TcpStream::connect((Ipv4Addr::LOCALHOST, puerto_t)).unwrap();
        cli.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        write!(
            cli,
            "POST /connect HTTP/1.1\r\nHost: localhost:{}\r\nOrigin: {origen}\r\nX-Sitfa-Code: {}\r\nContent-Length: 2\r\n\r\n{{}}",
            puente.puerto(),
            puente.estado.codigo,
        )
        .unwrap();
        let mut r_cli = BufReader::new(cli);
        let (st_cli, hdr_cli, _) = leer_respuesta_keep_alive(&mut r_cli).unwrap();
        assert_eq!(st_cli, 200);
        assert_eq!(hdr_cli.get("connection").map(String::as_str), Some("keep-alive"));
        std::thread::sleep(Duration::from_millis(160));
        assert!(leer_respuesta_keep_alive(&mut r_cli).is_err());
        hilo.join().unwrap();
    }
}
