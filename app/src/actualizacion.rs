//! Actualización en el lugar (E-I2): una corrección llega como un `flow_rust.exe` nuevo y reemplaza al que corre, sin
//! reinstalar nada más. Flow vigila `%LOCALAPPDATA%\FlowRust\actualizaciones`; si ahí hay un `flow_rust.exe` distinto del
//! que está corriendo, avisa, y solo con el botón verifica su huella SHA-256 contra `SHA256.txt` (el que entrega la CI),
//! deja el programa en uso como `flow_rust.exe.anterior` (Windows permite renombrar un ejecutable en uso), copia el nuevo
//! en su lugar y se reinicia. La extensión de Chrome, los datos, la sesión y la configuración no se tocan: viven en otras
//! carpetas y la extensión solo cambia cuando cambia su propio código. Nada se descarga: el archivo lo deja la persona.

use sha2::{Digest, Sha256};
use std::{io::{self, Read}, path::{Path, PathBuf}};

const MAX_EXE: u64 = 256 * 1024 * 1024;
const MAX_SHA: u64 = 64 * 1024;
const MAX_SIG: u64 = 4 * 1024;

pub const NOMBRE: &str = "flow_rust.exe";
pub const NOMBRE_FIRMA: &str = "SHA256.txt.sig";

/// Estado de la firma separada (`SHA256.txt.sig`), preparada para verificación Ed25519.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EstadoFirma {
    /// No existe `SHA256.txt.sig` (modo actual sin firma).
    Ausente,
    /// Existe `SHA256.txt.sig` con formato Ed25519 válido (64 bytes crudos o 128 hex), sin clave pública configurada.
    PresenteSinClave { firma_hex: String },
    /// Verificada con la clave pública proporcionada.
    Verificada,
}

pub const AVISO_SINCRONIZANDO: &str =
    "La actualización de la carpeta adicional todavía se está sincronizando; vuelve a intentar en unos minutos.";

pub const PAUSA_SINCRONIZACION: std::time::Duration = std::time::Duration::from_secs(2);

/// Una versión nueva lista para aplicar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pendiente {
    pub ruta: PathBuf,
    pub huella: String,
    /// El commit que declara `SHA256.txt`, si lo trae.
    pub commit: String,
}

/// Carpeta donde se deja el ejecutable nuevo.
pub fn carpeta() -> PathBuf {
    flow_rust::descargador::extension::carpeta_base().join("actualizaciones")
}

pub fn huella(ruta: &Path) -> Result<String, String> {
    huella_archivo(ruta, false)
}

fn huella_archivo(ruta: &Path, mz: bool) -> Result<String, String> {
    let mut f = std::fs::File::open(ruta).map_err(|e| format!("No se pudo leer {}: {e}", ruta.display()))?;
    let mut sha = Sha256::new();
    let prefijo = if mz {
        let mut bytes = [0; 2];
        if f.read_exact(&mut bytes).is_err() || &bytes != b"MZ" {
            return Err(format!("{NOMBRE} no parece un ejecutable de Windows; vuelve a copiarlo desde la entrega de la CI."));
        }
        sha.update(bytes);
        2
    } else { 0 };
    let n = io::copy(&mut f.take(MAX_EXE - prefijo + 1), &mut sha)
        .map_err(|e| format!("No se pudo calcular la huella: {e}"))?;
    if n + prefijo > MAX_EXE {
        return Err("El ejecutable supera el límite de 256 MiB.".into());
    }
    Ok(crate::trabajo::hex(&sha.finalize()))
}

/// `SHA256.txt` de la CI: «<huella>  flow_rust.exe (…)» y una línea «commit <sha>».
fn declarada_archivo(ruta: &Path) -> (Option<String>, String) {
    let mut texto = String::new();
    let Ok(f) = std::fs::File::open(ruta) else { return (None, String::new()) };
    if f.take(MAX_SHA + 1).read_to_string(&mut texto).is_err() || texto.len() as u64 > MAX_SHA {
        return (None, String::new());
    }
    let mut huella: Option<String> = None;
    for linea in texto.trim_start_matches('\u{feff}').lines() {
        let mut partes = linea.split_whitespace();
        let Some(h) = partes.next() else { continue };
        if h.len() != 64 || !h.bytes().all(|b| b.is_ascii_hexdigit()) { continue; }
        let resto = partes.collect::<Vec<_>>().join(" ");
        if !resto.is_empty() && !matches!(resto.trim_start_matches('*'), "flow_rust.exe" | "flow_rust.exe (debug)") {
            continue;
        }
        let h = h.to_lowercase();
        if huella.as_ref().is_some_and(|previa| previa != &h) { return (None, String::new()); }
        huella = Some(h);
    }
    let commit = texto.lines().find_map(|l| l.strip_prefix("commit ")).map(|c| c.trim().to_string()).unwrap_or_default();
    (huella, commit)
}

fn declarada(carpeta: &Path) -> (Option<String>, String) {
    declarada_archivo(&carpeta.join("SHA256.txt"))
}

fn rechazar_enlace_o_especial(ruta: &Path, permitir_ausente: bool) -> Result<bool, String> {
    match std::fs::symlink_metadata(ruta) {
        Ok(meta) => {
            #[cfg(windows)]
            let es_reparse = {
                use std::os::windows::fs::MetadataExt;
                meta.file_attributes() & 0x0400 != 0
            };
            #[cfg(not(windows))]
            let es_reparse = false;
            if meta.file_type().is_symlink() || es_reparse || !meta.file_type().is_file() {
                return Err(format!(
                    "Se rechazó la actualización porque {} es un enlace o no es un archivo regular.",
                    ruta.display()
                ));
            }
            Ok(true)
        }
        Err(e) if permitir_ausente && e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(format!("No se pudo inspeccionar {}: {e}", ruta.display())),
    }
}

fn decodificar_firma_ed25519(crudo: &[u8]) -> Result<[u8; 64], String> {
    if crudo.len() == 64 {
        let mut out = [0u8; 64];
        out.copy_from_slice(crudo);
        return Ok(out);
    }
    let texto = std::str::from_utf8(crudo)
        .map_err(|_| "SHA256.txt.sig no tiene un formato de firma Ed25519 válido.".to_string())?
        .trim_start_matches('\u{feff}')
        .trim();
    if texto.len() != 128 || !texto.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("SHA256.txt.sig no tiene un formato de firma Ed25519 válido (64 bytes o 128 hex).".into());
    }
    let mut out = [0u8; 64];
    for i in 0..64 {
        out[i] = u8::from_str_radix(&texto[i * 2..i * 2 + 2], 16)
            .map_err(|_| "SHA256.txt.sig contiene caracteres hexadecimales inválidos.".to_string())?;
    }
    Ok(out)
}

/// Gancho de verificación de firma separada (`SHA256.txt.sig`) preparado para Ed25519.
/// Valida tipo de archivo regular, tamaño acotado y formato (64 bytes crudos o 128 hex); si se entrega un verificador
/// criptográfico, valida la firma contra el contenido de `SHA256.txt`.
pub fn verificar_firma_opcional(
    carpeta: &Path,
    verificador: Option<&dyn Fn(&[u8], &[u8; 64]) -> bool>,
) -> Result<EstadoFirma, String> {
    let ruta_firma = carpeta.join(NOMBRE_FIRMA);
    if !rechazar_enlace_o_especial(&ruta_firma, true)? {
        return Ok(EstadoFirma::Ausente);
    }
    let f = std::fs::File::open(&ruta_firma)
        .map_err(|e| format!("No se pudo leer {}: {e}", ruta_firma.display()))?;
    let mut crudo = Vec::new();
    f.take(MAX_SIG + 1)
        .read_to_end(&mut crudo)
        .map_err(|e| format!("No se pudo leer {}: {e}", ruta_firma.display()))?;
    if crudo.len() as u64 > MAX_SIG {
        return Err("SHA256.txt.sig supera el tamaño permitido.".into());
    }
    let firma = decodificar_firma_ed25519(&crudo)?;
    if let Some(verificar) = verificador {
        let ruta_sha = carpeta.join("SHA256.txt");
        rechazar_enlace_o_especial(&ruta_sha, false)?;
        let f_sha = std::fs::File::open(&ruta_sha)
            .map_err(|e| format!("No se pudo leer {}: {e}", ruta_sha.display()))?;
        let mut sha_bytes = Vec::new();
        f_sha
            .take(MAX_SHA + 1)
            .read_to_end(&mut sha_bytes)
            .map_err(|e| format!("No se pudo leer {}: {e}", ruta_sha.display()))?;
        if sha_bytes.len() as u64 > MAX_SHA {
            return Err("SHA256.txt supera el tamaño permitido.".into());
        }
        if !verificar(&sha_bytes, &firma) {
            return Err("La firma criptográfica en SHA256.txt.sig no es válida.".into());
        }
        return Ok(EstadoFirma::Verificada);
    }
    Ok(EstadoFirma::PresenteSinClave { firma_hex: crate::trabajo::hex(&firma) })
}

/// La versión nueva que espera en `carpeta`, si la hay y difiere del programa que corre (`actual`).
pub fn buscar(carpeta: &Path, actual: &Path) -> Result<Option<Pendiente>, String> {
    let nuevo = carpeta.join(NOMBRE);
    if !nuevo.is_file() {
        return Ok(None);
    }
    let h = huella_archivo(&nuevo, true)?;
    let (esperada, commit) = declarada(carpeta);
    if esperada.as_deref() != Some(h.as_str()) {
        return Err("Falta SHA256.txt válido o la huella de la actualización no coincide: no se aplicó.".into());
    }
    verificar_firma_opcional(carpeta, None)?;
    if huella(actual).is_ok_and(|a| a == h) {
        return Ok(None);
    }
    Ok(Some(Pendiente { ruta: nuevo, huella: h, commit }))
}

/// Comprueba si el archivo está disponible localmente (no es un marcador de solo nube de OneDrive)
/// y se puede abrir para lectura.
#[cfg(windows)]
fn archivo_disponible_localmente(ruta: &Path) -> bool {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    if let Ok(meta) = std::fs::metadata(ruta) {
        let attrs = meta.file_attributes();
        // FILE_ATTRIBUTE_OFFLINE (0x1000)
        // FILE_ATTRIBUTE_RECALL_ON_OPEN (0x00040000)
        // FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS (0x00400000)
        if attrs & (0x1000 | 0x0004_0000 | 0x0040_0000) != 0 {
            return false;
        }
    }
    std::fs::OpenOptions::new().read(true).share_mode(0).open(ruta).is_ok()
}

#[cfg(not(windows))]
fn archivo_disponible_localmente(ruta: &Path) -> bool {
    std::fs::File::open(ruta).is_ok()
}

/// Comprueba que el tamaño y la fecha de modificación del archivo permanezcan estables entre dos lecturas.
fn verificar_estabilidad(ruta: &Path, pausa: std::time::Duration) -> bool {
    let Ok(m1) = std::fs::metadata(ruta) else { return false };
    let len1 = m1.len();
    let mod1 = m1.modified().ok();
    if !pausa.is_zero() {
        std::thread::sleep(pausa);
    }
    let Ok(m2) = std::fs::metadata(ruta) else { return false };
    let len2 = m2.len();
    let mod2 = m2.modified().ok();
    len1 == len2 && mod1 == mod2
}

/// Copia `flow_rust.exe` y `SHA256.txt` desde la carpeta adicional a la carpeta fija
/// usando archivos temporales y renombrado atómico para no sobrescribir a medias.
fn copiar_de_adicional_a_fija(
    fija: &Path,
    exe_origen: &Path,
    sha_origen: &Path,
    huella_esperada: &str,
) -> Result<(), String> {
    std::fs::create_dir_all(fija)
        .map_err(|e| format!("No se pudo crear la carpeta fija de actualizaciones {}: {e}", fija.display()))?;

    // Directorio exclusivo en el mismo volumen; no borrar temporales de otros procesos.
    let mut nonce = [0u8; 16];
    getrandom::fill(&mut nonce).map_err(|e| format!("No se pudo generar un temporal: {e}"))?;
    let transaccion = fija.join(format!(".flow-actualizacion-{}", crate::trabajo::hex(&nonce)));
    std::fs::create_dir(&transaccion).map_err(|e| format!("No se pudo crear el temporal: {e}"))?;
    let tmp_exe = transaccion.join("nuevo.exe");
    let tmp_sha = transaccion.join("nuevo.sha");
    let copia = (|| -> Result<(), String> {
        copiar_limitado(exe_origen, &tmp_exe, MAX_EXE)?;
        if huella_archivo(&tmp_exe, true)? != huella_esperada {
            return Err(AVISO_SINCRONIZANDO.into());
        }
        copiar_limitado(sha_origen, &tmp_sha, MAX_SHA)?;
        if declarada_archivo(&tmp_sha).0.as_deref() != Some(huella_esperada) {
            return Err(AVISO_SINCRONIZANDO.into());
        }
        publicar_pareja(fija, &transaccion, |a, b| std::fs::rename(a, b))
    })();
    // Si la restauración falla, conservar respaldos y su ubicación para recuperación manual.
    for p in [&tmp_exe, &tmp_sha] {
        let _ = std::fs::remove_file(p);
    }
    let _ = std::fs::remove_dir(&transaccion);
    copia
}

fn copiar_limitado(origen: &Path, destino: &Path, max: u64) -> Result<(), String> {
    let f = std::fs::File::open(origen).map_err(|e| format!("No se pudo abrir {}: {e}", origen.display()))?;
    let mut out = std::fs::OpenOptions::new().write(true).create_new(true).open(destino)
        .map_err(|e| format!("No se pudo crear {}: {e}", destino.display()))?;
    let n = io::copy(&mut f.take(max + 1), &mut out).map_err(|e| format!("No se pudo copiar: {e}"))?;
    if n > max { return Err("La actualización excede el límite de tamaño.".into()); }
    out.sync_all().map_err(|e| format!("No se pudo guardar la copia: {e}"))
}

// Separa respaldo y publicación para poder inyectar fallos de renombrado en pruebas.
fn publicar_pareja(
    fija: &Path,
    transaccion: &Path,
    mut mover: impl FnMut(&Path, &Path) -> io::Result<()>,
) -> Result<(), String> {
    let destinos = [fija.join(NOMBRE), fija.join("SHA256.txt")];
    let respaldos = [transaccion.join("previo.exe"), transaccion.join("previo.sha")];
    let nuevos = [transaccion.join("nuevo.exe"), transaccion.join("nuevo.sha")];
    let mut respaldado = [false; 2];
    let mut publicado = [false; 2];
    let operacion = (|| -> io::Result<()> {
        // Retirar primero la huella: nunca ofrecer una pareja parcialmente publicada.
        for i in [1, 0] {
            match std::fs::symlink_metadata(&destinos[i]) {
                Ok(m) if m.file_type().is_file() => {
                    mover(&destinos[i], &respaldos[i])?;
                    respaldado[i] = true;
                }
                Ok(_) => return Err(io::Error::other("Destino de actualización no es un archivo regular")),
                Err(e) if e.kind() == io::ErrorKind::NotFound => {},
                Err(e) => return Err(e),
            }
        }
        for i in [0, 1] {
            mover(&nuevos[i], &destinos[i])?;
            publicado[i] = true;
        }
        Ok(())
    })();
    if let Err(e) = operacion {
        let mut fallos = Vec::new();
        for i in [1, 0] {
            if publicado[i] {
                if let Err(e) = mover(&destinos[i], &nuevos[i]) { fallos.push(e.to_string()); }
            }
        }
        for i in [0, 1] {
            if respaldado[i] {
                if destinos[i].exists() {
                    fallos.push(format!("Destino ocupado: {}", destinos[i].display()));
                } else if let Err(e) = mover(&respaldos[i], &destinos[i]) { fallos.push(e.to_string()); }
            }
        }
        if !fallos.is_empty() {
            return Err(format!("No se pudo publicar: {e}. Restauración incompleta en {}: {}", transaccion.display(), fallos.join("; ")));
        }
        return Err(format!("No se pudo publicar la actualización; se restauró el estado previo: {e}"));
    }
    for i in [0, 1] {
        if respaldado[i] { let _ = std::fs::remove_file(&respaldos[i]); }
    }
    Ok(())
}

/// Busca en la carpeta fija y, si está configurada, en la adicional con una pausa configurable para la verificación de sincronización.
pub fn buscar_con_adicional_pausa(
    fija: &Path,
    adicional: Option<&Path>,
    actual: &Path,
    pausa: std::time::Duration,
) -> Result<Option<Pendiente>, String> {
    if let Some(adicional) = adicional.filter(|p| !p.as_os_str().is_empty()) {
        let nuevo_adicional = adicional.join(NOMBRE);
        if nuevo_adicional.is_file() {
            let h_actual = huella(actual).ok();

            // (c) Verificar si el archivo se puede abrir en lectura exclusiva y no es marcador sólo en la nube
            if !archivo_disponible_localmente(&nuevo_adicional) {
                return Err(AVISO_SINCRONIZANDO.into());
            }

            // (b) Verificar si el tamaño o fecha de modificación cambió entre dos lecturas separadas por al menos 2s
            if !verificar_estabilidad(&nuevo_adicional, pausa) {
                return Err(AVISO_SINCRONIZANDO.into());
            }

            let h_nuevo = huella_archivo(&nuevo_adicional, true)?;

            // (a) Falta SHA256.txt o no coincide
            let sha_adicional = adicional.join("SHA256.txt");
            if !sha_adicional.is_file() {
                return Err(AVISO_SINCRONIZANDO.into());
            }
            let (esperada, _) = declarada(adicional);
            if esperada.as_deref() != Some(&h_nuevo) {
                return Err(AVISO_SINCRONIZANDO.into());
            }

            if h_actual.as_deref() == Some(h_nuevo.as_str()) {
                return buscar(fija, actual);
            }

            // Copiar a la fija (por temporal y renombrado, nunca sobrescribe a medias)
            copiar_de_adicional_a_fija(fija, &nuevo_adicional, &sha_adicional, &h_nuevo)?;
        }
    }

    buscar(fija, actual)
}

/// Busca en la carpeta fija (%LOCALAPPDATA%\FlowRust\actualizaciones) y, si está configurada, en la adicional.
pub fn buscar_con_adicional(
    fija: &Path,
    adicional: Option<&Path>,
    actual: &Path,
) -> Result<Option<Pendiente>, String> {
    buscar_con_adicional_pausa(fija, adicional, actual, PAUSA_SINCRONIZACION)
}

fn anterior_de(actual: &Path) -> PathBuf {
    let nombre = actual.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| NOMBRE.into());
    actual.with_file_name(format!("{nombre}.anterior"))
}

fn anterior_bak_de(actual: &Path) -> PathBuf {
    let nombre = actual.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| NOMBRE.into());
    actual.with_file_name(format!("{nombre}.anterior.bak"))
}

/// La copia del programa anterior, si existe (para volver atrás).
pub fn anterior(actual: &Path) -> Option<PathBuf> {
    Some(anterior_de(actual)).filter(|p| p.is_file())
}

/// Reemplaza `actual` por la versión pendiente. Si algo falla antes de terminar, el programa queda como estaba.
pub fn aplicar(p: &Pendiente, actual: &Path) -> Result<PathBuf, String> {
    aplicar_con(p, actual, |a, b| std::fs::rename(a, b))
}

fn aplicar_con(
    p: &Pendiente,
    actual: &Path,
    mut mover: impl FnMut(&Path, &Path) -> io::Result<()>,
) -> Result<PathBuf, String> {
    rechazar_enlace_o_especial(&p.ruta, false)?;
    rechazar_enlace_o_especial(actual, false)?;
    if huella_archivo(&p.ruta, true)? != p.huella ||
        p.ruta.parent().and_then(|dir| declarada(dir).0).as_deref() != Some(p.huella.as_str()) {
        return Err("La actualización cambió después de verificarse; vuelve a buscarla.".into());
    }
    if let Some(carpeta) = p.ruta.parent() {
        verificar_firma_opcional(carpeta, None)?;
    }
    let anterior = anterior_de(actual);
    let anterior_bak = anterior_bak_de(actual);
    let habia_anterior = rechazar_enlace_o_especial(&anterior, true)?;
    if rechazar_enlace_o_especial(&anterior_bak, true)? {
        let _ = std::fs::remove_file(&anterior_bak);
    }
    let sin_permiso = |e: std::io::Error| {
        format!("No se pudo reemplazar el programa en {}: {e}. Mueve Flow a una carpeta propia, por ejemplo %LOCALAPPDATA%\\FlowRust.", actual.display())
    };
    if habia_anterior {
        mover(&anterior, &anterior_bak).map_err(sin_permiso)?;
    }
    if let Err(e) = mover(actual, &anterior) {
        if habia_anterior {
            let _ = mover(&anterior_bak, &anterior);
        }
        return Err(sin_permiso(e));
    }
    let mut restaurar = |motivo: String| {
        let _ = std::fs::remove_file(actual);
        let _ = mover(&anterior, actual);
        if habia_anterior {
            let _ = mover(&anterior_bak, &anterior);
        }
        motivo
    };
    if let Err(e) = copiar_limitado(&p.ruta, actual, MAX_EXE) {
        return Err(restaurar(format!("No se pudo copiar la actualización: {e}")));
    }
    if huella(actual).ok().as_deref() != Some(p.huella.as_str()) {
        return Err(restaurar("La copia del programa nuevo no coincide con la verificada; se conservó el anterior.".into()));
    }
    if habia_anterior {
        let _ = std::fs::remove_file(&anterior_bak);
    }
    // La actualización ya aplicada se guarda aparte para que no se vuelva a ofrecer.
    if let Some(carpeta) = p.ruta.parent() {
        let aplicadas = carpeta.join("aplicadas");
        if std::fs::create_dir_all(&aplicadas).is_ok() {
            let corto = &p.huella[..8.min(p.huella.len())];
            let _ = std::fs::rename(&p.ruta, aplicadas.join(format!("flow_rust-{corto}.exe")));
            let _ = std::fs::rename(carpeta.join("SHA256.txt"), aplicadas.join(format!("SHA256-{corto}.txt")));
            let sig = carpeta.join(NOMBRE_FIRMA);
            if sig.is_file() {
                let _ = std::fs::rename(&sig, aplicadas.join(format!("SHA256-{corto}.txt.sig")));
            }
        }
    }
    Ok(anterior)
}

/// Intercambia el programa actual con la copia anterior.
pub fn volver(actual: &Path) -> Result<(), String> {
    let anterior = anterior_de(actual);
    if !anterior.is_file() {
        return Err("No hay una versión anterior guardada.".into());
    }
    intercambiar(actual, &anterior, |a, b| std::fs::rename(a, b))
}

fn intercambiar(actual: &Path, anterior: &Path, mut mover: impl FnMut(&Path, &Path) -> io::Result<()>) -> Result<(), String> {
    let temporal = actual.with_file_name("flow_rust.exe.intercambio");
    if std::fs::symlink_metadata(&temporal).is_ok() {
        return Err("Hay un intercambio previo pendiente de recuperación; no se sobrescribió.".into());
    }
    let mut paso = |a: &Path, b: &Path| mover(a, b).map_err(|e| format!("No se pudo volver a la versión anterior: {e}"));
    paso(actual, &temporal)?;
    if let Err(e) = paso(anterior, actual) {
        return match paso(&temporal, actual) {
            Ok(()) => Err(e),
            Err(r) => Err(format!("{e}; restauración fallida: {r}; respaldo en {}", temporal.display())),
        };
    }
    if let Err(e) = paso(&temporal, anterior) {
        // Deshacer en orden inverso, sin destruir ninguna de las dos versiones.
        if let Err(r) = paso(actual, anterior) {
            return Err(format!("{e}; restauración fallida: {r}; respaldo en {}", temporal.display()));
        }
        if let Err(r) = paso(&temporal, actual) {
            return Err(format!("{e}; restauración fallida: {r}; respaldo en {}", temporal.display()));
        }
        return Err(e);
    }
    Ok(())
}

/// Abre el programa que quedó en `actual` (el nuevo) para que continúe el trabajo.
pub fn reiniciar(actual: &Path) -> Result<(), String> {
    std::process::Command::new(actual).spawn().map(|_| ()).map_err(|e| format!("No se pudo reiniciar Flow: {e}"))
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn carpeta_temporal(nombre: &str) -> PathBuf {
        let c = std::env::temp_dir().join(format!("flow-actualizacion-{}-{nombre}", std::process::id()));
        let _ = std::fs::remove_dir_all(&c);
        std::fs::create_dir_all(&c).unwrap();
        c
    }

    fn exe(ruta: &Path, contenido: &str) {
        std::fs::write(ruta, format!("MZ{contenido}")).unwrap();
    }

    #[test]
    fn la_actualizacion_reemplaza_el_programa_y_deja_el_anterior() {
        let base = carpeta_temporal("aplica");
        let (programa, nuevas) = (base.join("programa"), base.join("actualizaciones"));
        std::fs::create_dir_all(&programa).unwrap();
        std::fs::create_dir_all(&nuevas).unwrap();
        let actual = programa.join(NOMBRE);
        exe(&actual, "version vieja");
        // Sin nada en la carpeta no hay aviso.
        assert_eq!(buscar(&nuevas, &actual).unwrap(), None);
        exe(&nuevas.join(NOMBRE), "version nueva");
        let h = huella(&nuevas.join(NOMBRE)).unwrap();
        std::fs::write(nuevas.join("SHA256.txt"), format!("{h}  flow_rust.exe (debug)\ncommit abc1234\n")).unwrap();
        let pendiente = buscar(&nuevas, &actual).unwrap().expect("hay una versión nueva");
        assert_eq!((pendiente.huella.as_str(), pendiente.commit.as_str()), (h.as_str(), "abc1234"));
        // Buscar no cambia nada: solo avisa.
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion vieja");
        let anterior = aplicar(&pendiente, &actual).unwrap();
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion nueva");
        assert_eq!(std::fs::read(&anterior).unwrap(), b"MZversion vieja");
        assert!(!nuevas.join(NOMBRE).exists(), "la actualización aplicada no se vuelve a ofrecer");
        assert!(nuevas.join("aplicadas").is_dir());
        assert_eq!(buscar(&nuevas, &actual).unwrap(), None);
        // Volver atrás intercambia los dos y se puede repetir.
        volver(&actual).unwrap();
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion vieja");
        volver(&actual).unwrap();
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion nueva");
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn una_actualizacion_igual_incompleta_o_ajena_no_se_aplica() {
        let base = carpeta_temporal("rechaza");
        let nuevas = base.join("actualizaciones");
        std::fs::create_dir_all(&nuevas).unwrap();
        let actual = base.join(NOMBRE);
        exe(&actual, "version vieja");
        // Igual al programa que corre: está al día, con huella válida.
        exe(&nuevas.join(NOMBRE), "version vieja");
        assert!(buscar(&nuevas, &actual).is_err());
        let h = huella(&nuevas.join(NOMBRE)).unwrap();
        std::fs::write(nuevas.join("SHA256.txt"), format!("{h}  flow_rust.exe\n")).unwrap();
        assert_eq!(buscar(&nuevas, &actual).unwrap(), None);
        // La huella declarada no coincide: archivo incompleto o cambiado.
        exe(&nuevas.join(NOMBRE), "version nueva");
        std::fs::write(nuevas.join("SHA256.txt"), format!("{}  flow_rust.exe\n", "0".repeat(64))).unwrap();
        assert!(buscar(&nuevas, &actual).unwrap_err().contains("no coincide"));
        // No es un ejecutable.
        std::fs::remove_file(nuevas.join("SHA256.txt")).unwrap();
        std::fs::write(nuevas.join(NOMBRE), "texto").unwrap();
        assert!(buscar(&nuevas, &actual).unwrap_err().contains("ejecutable"));
        // Cambió entre la verificación y la aplicación: el programa queda como estaba.
        exe(&nuevas.join(NOMBRE), "version nueva");
        assert!(buscar(&nuevas, &actual).is_err());
        std::fs::write(nuevas.join("SHA256.txt"), "huella corrupta").unwrap();
        assert!(buscar(&nuevas, &actual).is_err());
        let h = huella(&nuevas.join(NOMBRE)).unwrap();
        std::fs::write(nuevas.join("SHA256.txt"), format!("{h}  flow_rust.exe\n")).unwrap();
        let pendiente = buscar(&nuevas, &actual).unwrap().unwrap();
        exe(&nuevas.join(NOMBRE), "otra cosa");
        assert!(aplicar(&pendiente, &actual).unwrap_err().contains("cambió"));
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion vieja");
        assert!(anterior(&actual).is_none());
        assert!(volver(&actual).is_err());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn la_actualizacion_de_la_carpeta_adicional_se_copia_y_aplica() {
        let base = carpeta_temporal("adicional-aplica");
        let (programa, fija, adicional) = (base.join("programa"), base.join("fija"), base.join("adicional"));
        std::fs::create_dir_all(&programa).unwrap();
        std::fs::create_dir_all(&fija).unwrap();
        std::fs::create_dir_all(&adicional).unwrap();

        let actual = programa.join(NOMBRE);
        exe(&actual, "version vieja");

        exe(&adicional.join(NOMBRE), "version nueva");
        let h = huella(&adicional.join(NOMBRE)).unwrap();
        std::fs::write(adicional.join("SHA256.txt"), format!("{h}  flow_rust.exe\ncommit def5678\n")).unwrap();

        assert!(!fija.join(NOMBRE).exists());
        let pendiente = buscar_con_adicional_pausa(&fija, Some(&adicional), &actual, std::time::Duration::ZERO)
            .unwrap()
            .expect("debe detectar la versión nueva de la carpeta adicional");

        assert_eq!((pendiente.huella.as_str(), pendiente.commit.as_str()), (h.as_str(), "def5678"));
        // Se copió a la fija antes de aplicar
        assert_eq!(std::fs::read(fija.join(NOMBRE)).unwrap(), b"MZversion nueva");
        assert!(fija.join("SHA256.txt").is_file());

        // Aplicar desde la fija
        let anterior = aplicar(&pendiente, &actual).unwrap();
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion nueva");
        assert_eq!(std::fs::read(&anterior).unwrap(), b"MZversion vieja");
        assert!(fija.join("aplicadas").is_dir());

        // Una búsqueda posterior ve que el programa en uso ya tiene esa versión y no vuelve a ofrecerla ni a copiarla
        assert_eq!(buscar_con_adicional_pausa(&fija, Some(&adicional), &actual, std::time::Duration::ZERO).unwrap(), None);

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn una_actualizacion_a_medio_sincronizar_no_se_aplica() {
        let base = carpeta_temporal("adicional-mediosync");
        let (programa, fija, adicional) = (base.join("programa"), base.join("fija"), base.join("adicional"));
        std::fs::create_dir_all(&programa).unwrap();
        std::fs::create_dir_all(&fija).unwrap();
        std::fs::create_dir_all(&adicional).unwrap();

        let actual = programa.join(NOMBRE);
        exe(&actual, "version vieja");

        exe(&adicional.join(NOMBRE), "version parcial");
        let h = huella(&adicional.join(NOMBRE)).unwrap();
        std::fs::write(adicional.join("SHA256.txt"), format!("{h}  flow_rust.exe\n")).unwrap();

        // Simular que el tamaño cambia durante la verificación (entre dos lecturas)
        let exe_adicional = adicional.join(NOMBRE);
        let h_thread = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            let mut f = std::fs::OpenOptions::new().append(true).open(&exe_adicional).unwrap();
            use std::io::Write;
            let _ = f.write_all(b"mas datos que cambian tamano");
        });

        let res = buscar_con_adicional_pausa(&fija, Some(&adicional), &actual, std::time::Duration::from_millis(150));
        h_thread.join().unwrap();

        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), AVISO_SINCRONIZANDO);
        assert!(!fija.join(NOMBRE).exists(), "no debe copiarse a la fija si está a medio sincronizar");
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion vieja");

        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn sin_sha256_en_la_carpeta_adicional_no_se_aplica() {
        let base = carpeta_temporal("adicional-sinsha");
        let (programa, fija, adicional) = (base.join("programa"), base.join("fija"), base.join("adicional"));
        std::fs::create_dir_all(&programa).unwrap();
        std::fs::create_dir_all(&fija).unwrap();
        std::fs::create_dir_all(&adicional).unwrap();

        let actual = programa.join(NOMBRE);
        exe(&actual, "version vieja");

        exe(&adicional.join(NOMBRE), "version nueva");
        // No se crea SHA256.txt en la carpeta adicional

        let res = buscar_con_adicional_pausa(&fija, Some(&adicional), &actual, std::time::Duration::ZERO);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), AVISO_SINCRONIZANDO);
        assert!(!fija.join(NOMBRE).exists(), "no debe copiarse a la fija si falta SHA256.txt");
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion vieja");

        // Caso con SHA256.txt que no coincide
        std::fs::write(adicional.join("SHA256.txt"), format!("{}  flow_rust.exe\n", "1".repeat(64))).unwrap();
        let res2 = buscar_con_adicional_pausa(&fija, Some(&adicional), &actual, std::time::Duration::ZERO);
        assert!(res2.is_err());
        assert_eq!(res2.unwrap_err(), AVISO_SINCRONIZANDO);
        assert!(!fija.join(NOMBRE).exists());

        let _ = std::fs::remove_dir_all(&base);
    }
    #[test]
    fn sha_copiada_de_otra_version_no_reemplaza_pareja_previa() {
        let base = carpeta_temporal("sha-copiada-ajena");
        let fija = base.join("fija");
        let adicional = base.join("adicional");
        std::fs::create_dir_all(&fija).unwrap();
        std::fs::create_dir_all(&adicional).unwrap();
        exe(&fija.join(NOMBRE), "previo");
        let h_previa = huella(&fija.join(NOMBRE)).unwrap();
        std::fs::write(fija.join("SHA256.txt"), format!("{h_previa}  flow_rust.exe\n")).unwrap();
        exe(&adicional.join(NOMBRE), "nuevo");
        let h_nueva = huella(&adicional.join(NOMBRE)).unwrap();
        let sha = adicional.join("SHA256.txt");
        std::fs::write(&sha, format!("{h_previa}  flow_rust.exe\n")).unwrap();
        assert!(copiar_de_adicional_a_fija(&fija, &adicional.join(NOMBRE), &sha, &h_nueva).is_err());
        assert_eq!(huella(&fija.join(NOMBRE)).unwrap(), h_previa);
        assert_eq!(declarada(&fija).0.as_deref(), Some(h_previa.as_str()));
        assert_eq!(std::fs::read_dir(&fija).unwrap().count(), 2);
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn falla_segunda_publicacion_restaura_pareja_previa() {
        let base = carpeta_temporal("rollback-pareja");
        let tx = base.join("tx");
        std::fs::create_dir(&tx).unwrap();
        exe(&base.join(NOMBRE), "previo");
        let previo = huella(&base.join(NOMBRE)).unwrap();
        std::fs::write(base.join("SHA256.txt"), &previo).unwrap();
        exe(&tx.join("nuevo.exe"), "nuevo");
        std::fs::write(tx.join("nuevo.sha"), "nuevo").unwrap();
        let resultado = publicar_pareja(&base, &tx, |a, b| {
            if a == tx.join("nuevo.sha").as_path() { Err(io::Error::other("fallo inyectado")) }
            else { std::fs::rename(a, b) }
        });
        assert!(resultado.is_err());
        assert_eq!(huella(&base.join(NOMBRE)).unwrap(), previo);
        assert_eq!(std::fs::read_to_string(base.join("SHA256.txt")).unwrap(), previo);
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn falla_intercambio_final_restaura_ambas_versiones() {
        let base = carpeta_temporal("rollback-intercambio");
        let actual = base.join(NOMBRE);
        let anterior = anterior_de(&actual);
        exe(&actual, "actual");
        exe(&anterior, "anterior");
        let mut pasos = 0;
        let resultado = intercambiar(&actual, &anterior, |a, b| {
            pasos += 1;
            if pasos == 3 { Err(io::Error::other("fallo inyectado")) }
            else { std::fs::rename(a, b) }
        });
        assert!(resultado.is_err());
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZactual");
        assert_eq!(std::fs::read(&anterior).unwrap(), b"MZanterior");
        assert!(!base.join("flow_rust.exe.intercambio").exists());
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn aplicar_preserva_anterior_si_falla_renombrado_y_rechaza_no_regular() {
        let base = carpeta_temporal("rollback-anterior");
        let (programa, nuevas) = (base.join("programa"), base.join("actualizaciones"));
        std::fs::create_dir_all(&programa).unwrap();
        std::fs::create_dir_all(&nuevas).unwrap();
        let actual = programa.join(NOMBRE);
        let anterior = anterior_de(&actual);
        exe(&actual, "version actual");
        exe(&anterior, "version previa");
        exe(&nuevas.join(NOMBRE), "version nueva");
        let h = huella(&nuevas.join(NOMBRE)).unwrap();
        std::fs::write(nuevas.join("SHA256.txt"), format!("{h}  flow_rust.exe\n")).unwrap();
        let pendiente = buscar(&nuevas, &actual).unwrap().unwrap();

        // Si falla mover `actual` a `anterior`, el respaldo previo debe restaurarse intacto.
        let res = aplicar_con(&pendiente, &actual, |a, b| {
            if a == actual.as_path() && b == anterior.as_path() {
                Err(io::Error::other("fallo inyectado"))
            } else {
                std::fs::rename(a, b)
            }
        });
        assert!(res.is_err());
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion actual");
        assert_eq!(std::fs::read(&anterior).unwrap(), b"MZversion previa");
        assert!(!anterior_bak_de(&actual).exists());

        // Rechaza `anterior` que no sea archivo regular (p. ej. directorio o enlace).
        std::fs::remove_file(&anterior).unwrap();
        std::fs::create_dir(&anterior).unwrap();
        assert!(aplicar(&pendiente, &actual).unwrap_err().contains("enlace o no es un archivo regular"));
        assert_eq!(std::fs::read(&actual).unwrap(), b"MZversion actual");
        std::fs::remove_dir(&anterior).unwrap();

        // Gancho de firma separada SHA256.txt.sig (ausente, formato válido, inválido y verificador).
        assert_eq!(verificar_firma_opcional(&nuevas, None).unwrap(), EstadoFirma::Ausente);
        std::fs::write(nuevas.join(NOMBRE_FIRMA), "firma-invalida").unwrap();
        assert!(verificar_firma_opcional(&nuevas, None).is_err());
        let firma_hex = "ab".repeat(64);
        std::fs::write(nuevas.join(NOMBRE_FIRMA), format!("{firma_hex}\n")).unwrap();
        assert_eq!(
            verificar_firma_opcional(&nuevas, None).unwrap(),
            EstadoFirma::PresenteSinClave { firma_hex: firma_hex.clone() }
        );
        assert_eq!(
            verificar_firma_opcional(&nuevas, Some(&|_msg, sig| sig[0] == 0xab)).unwrap(),
            EstadoFirma::Verificada
        );
        assert!(verificar_firma_opcional(&nuevas, Some(&|_msg, _sig| false)).is_err());

        let _ = std::fs::remove_dir_all(base);
    }

}
