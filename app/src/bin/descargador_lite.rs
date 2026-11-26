//! Descargador SITFA Lite: aplicación de escritorio enfocada únicamente en
//! Seguimiento, Búsqueda de litigantes, Informes por vencer y Medidas por vencer,
//! con pantallazo PDF ante consultas sin registros, etiquetado oficial de archivos
//! y consolidación de descargas en Excel (.xlsx).
//!
//! Solo lectura: nunca modifica registros en SITFA ni RUS.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::{Local, NaiveDate};
use egui::{Color32, CornerRadius, Frame, Margin, RichText, ScrollArea, Stroke, Ui};
use flow_rust::descargador::extension::{self, Cambio};
use flow_rust::descargador::lote::{self, Evento, Fin, Opciones};
use flow_rust::descargador::paquete::{self, Lote};
use flow_rust::descargador::resultados;
use flow_rust::descargador::{
    Catalogo, Edicion, Pantalla, PlanLote, Puente, SeleccionLote, MODALIDADES, PESTANAS,
    TODAS_LAS_MEDIDAS,
};
use flow_rust::theme::{self, Pal};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::time::Duration;

const TITULO_VENTANA: &str = "Descargador SITFA Lite";
const TAMANO_INICIAL: [f32; 2] = [1220.0, 820.0];
const TAMANO_MINIMO: [f32; 2] = [940.0, 620.0];

/// Pantallas habilitadas en esta edición (las 4 permitidas).
const PANTALLAS_PERMITIDAS: [Pantalla; 4] = Pantalla::LITE;

/// Comparte exactamente la misma extensión de Chrome que la versión completa
/// (`%LOCALAPPDATA%\FlowRust\extension-integral`, versión 2.10.0), sin exigir
/// una segunda extensión instalada en el navegador.
const EXTENSION_COMPARTIDA: Edicion = Edicion::Integral;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Vista {
    Descargar,
    Consolidar,
}

enum MensajeCatalogo {
    Listo(Pantalla, Result<Catalogo, String>),
}

enum MensajeDescarga {
    Evento(Evento),
    Terminado(Result<PathBuf, String>),
}

enum MensajeConsolidacion {
    Listo(Result<PathBuf, String>),
}

struct CarpetaConsolidacion {
    ruta: PathBuf,
    resumen: String,
    parcial: bool,
    valida: bool,
}

struct LiteApp {
    vista: Vista,
    oscuro: bool,
    tema_aplicado: Option<bool>,

    // Paso 1: conexión con Chrome y extensión compartida
    puente: Option<Arc<Puente>>,
    aviso_extension: Option<(String, bool)>,

    // Paso 2: pantalla y filtros
    pantalla: Pantalla,
    catalogo: Option<Catalogo>,
    busqueda_tribunal: String,
    tribunales: BTreeSet<String>,
    pestanas: BTreeSet<String>,
    modalidades: BTreeSet<String>,
    informes: BTreeSet<String>,
    medidas: BTreeSet<String>,
    estados: BTreeSet<String>,
    meses: BTreeSet<String>,
    anio: String,
    todas_las_fechas: bool,
    desde_texto: String,
    hasta_texto: String,
    conservar_filtros: bool,

    // Paso 3: descarga
    destino: PathBuf,
    evidencia_pdf: bool,
    cancelar: Arc<AtomicBool>,
    progreso_actual: usize,
    progreso_total: usize,
    estado_actual: String,
    eventos: Vec<String>,
    ultimo_fin: Option<Fin>,
    ultimo_error: Option<String>,
    pendientes_ruta: Option<PathBuf>,

    // Consolidación en Excel
    carpetas_consolidar: Vec<CarpetaConsolidacion>,
    permitir_incompletos: bool,
    destino_excel: Option<PathBuf>,
    ultimo_excel: Option<PathBuf>,
    error_consolidar: Option<String>,

    // Tareas en segundo plano
    rx_catalogo: Option<Receiver<MensajeCatalogo>>,
    rx_descarga: Option<Receiver<MensajeDescarga>>,
    rx_consolidar: Option<Receiver<MensajeConsolidacion>>,
}

impl LiteApp {
    fn nueva() -> Self {
        let destino = carpeta_descargas_por_defecto();
        Self {
            vista: Vista::Descargar,
            oscuro: false,
            tema_aplicado: None,
            puente: None,
            aviso_extension: None,
            pantalla: Pantalla::Seguimiento,
            catalogo: None,
            busqueda_tribunal: String::new(),
            tribunales: BTreeSet::new(),
            pestanas: BTreeSet::new(),
            modalidades: BTreeSet::new(),
            informes: BTreeSet::new(),
            medidas: BTreeSet::new(),
            estados: BTreeSet::new(),
            meses: BTreeSet::new(),
            anio: String::new(),
            todas_las_fechas: true,
            desde_texto: String::new(),
            hasta_texto: String::new(),
            conservar_filtros: false,
            destino,
            evidencia_pdf: true,
            cancelar: Arc::new(AtomicBool::new(false)),
            progreso_actual: 0,
            progreso_total: 0,
            estado_actual: String::new(),
            eventos: Vec::new(),
            ultimo_fin: None,
            ultimo_error: None,
            pendientes_ruta: None,
            carpetas_consolidar: Vec::new(),
            permitir_incompletos: false,
            destino_excel: None,
            ultimo_excel: None,
            error_consolidar: None,
            rx_catalogo: None,
            rx_descarga: None,
            rx_consolidar: None,
        }
    }

    fn ocupado_descarga(&self) -> bool {
        self.rx_descarga.is_some()
    }

    fn ocupado_catalogo(&self) -> bool {
        self.rx_catalogo.is_some()
    }

    fn ocupado_consolidar(&self) -> bool {
        self.rx_consolidar.is_some()
    }

    fn instalar_extension(&mut self) {
        let base = extension::carpeta_base();
        match extension::instalar_en_el_lugar(EXTENSION_COMPARTIDA, &base) {
            Ok(inst) => {
                let ruta = inst.carpeta.display().to_string();
                let mensaje = match inst.cambio {
                    Cambio::Nueva => format!(
                        "Extensión instalada en {ruta}. En Chrome abre chrome://extensions, activa «Modo de desarrollador» y elige «Cargar sin empaquetar»."
                    ),
                    Cambio::Actualizada { de, a } => format!(
                        "Extensión actualizada ({de} → {a}) en {ruta}. En Chrome basta pulsar «Recargar» en la extensión."
                    ),
                    Cambio::SinCambios => format!(
                        "La extensión (v{}) ya está al día en {ruta}.",
                        extension::version(EXTENSION_COMPARTIDA)
                    ),
                };
                self.aviso_extension = Some((mensaje, false));
            }
            Err(e) => {
                self.aviso_extension = Some((e, true));
            }
        }
    }

    fn iniciar_conexion(&mut self) {
        if self.puente.is_some() {
            return;
        }
        match Puente::abrir() {
            Ok(p) => {
                self.puente = Some(Arc::new(p));
                self.ultimo_error = None;
            }
            Err(e) => {
                self.ultimo_error = Some(e);
            }
        }
    }

    fn cerrar_conexion(&mut self) {
        if let Some(p) = self.puente.take() {
            p.cerrar();
        }
        self.catalogo = None;
    }

    fn cargar_catalogo(&mut self) {
        let Some(puente) = self.puente.clone() else {
            self.ultimo_error = Some("Inicia la conexión con Chrome antes de cargar opciones.".into());
            return;
        };
        if !PANTALLAS_PERMITIDAS.contains(&self.pantalla) {
            self.ultimo_error = Some("Pantalla no admitida.".into());
            return;
        }
        let pantalla = self.pantalla;
        self.ultimo_error = None;
        self.estado_actual = format!("Cargando opciones de {}…", pantalla.etiqueta());
        let (tx, rx) = mpsc::channel();
        self.rx_catalogo = Some(rx);
        std::thread::spawn(move || {
            let res = puente.catalogo(EXTENSION_COMPARTIDA, pantalla);
            let _ = tx.send(MensajeCatalogo::Listo(pantalla, res));
        });
    }

    fn aplicar_catalogo(&mut self, pantalla: Pantalla, cat: Catalogo) {
        self.pantalla = pantalla;
        self.tribunales.retain(|k| cat.tribunales.contains_key(k));
        self.pestanas.retain(|p| cat.pestanas.contains(p));
        self.modalidades.retain(|m| cat.modalidades.contains_key(m));
        self.informes.retain(|i| cat.informes.contains_key(i));
        self.medidas
            .retain(|m| m == TODAS_LAS_MEDIDAS || cat.medidas.contains_key(m));
        self.estados.retain(|e| cat.estados.contains_key(e));
        self.meses.retain(|m| cat.meses.contains_key(m));
        if !cat.anios.contains_key(&self.anio) {
            self.anio = cat.anios.keys().next_back().cloned().unwrap_or_default();
        }
        if self.medidas.is_empty() && pantalla == Pantalla::Seguimiento {
            self.medidas.insert(TODAS_LAS_MEDIDAS.into());
        }
        self.catalogo = Some(cat);
        self.estado_actual = format!("Opciones de {} cargadas.", pantalla.etiqueta());
    }

    fn seleccion_actual(&self, cat: &Catalogo) -> Result<SeleccionLote, String> {
        let fechas = if usa_fechas(cat.pantalla) && !self.todas_las_fechas {
            let desde = parsear_fecha(&self.desde_texto)
                .ok_or_else(|| "Ingresa una fecha «Desde» válida (dd/mm/aaaa).".to_string())?;
            let hasta = parsear_fecha(&self.hasta_texto)
                .ok_or_else(|| "Ingresa una fecha «Hasta» válida (dd/mm/aaaa).".to_string())?;
            if desde > hasta {
                return Err("La fecha «Desde» no puede ser posterior a «Hasta».".into());
            }
            Some((desde, hasta))
        } else {
            None
        };

        let tribunales: Vec<String> = cat
            .tribunales
            .keys()
            .filter(|k| self.tribunales.contains(*k))
            .cloned()
            .collect();
        let pestanas: Vec<String> = PESTANAS
            .iter()
            .filter(|p| self.pestanas.contains(**p))
            .map(|p| (*p).to_string())
            .collect();
        let modalidades: Vec<String> = MODALIDADES
            .iter()
            .filter(|m| self.modalidades.contains(**m))
            .map(|m| (*m).to_string())
            .collect();
        let informes: Vec<String> = cat
            .informes
            .keys()
            .filter(|k| self.informes.contains(*k))
            .cloned()
            .collect();
        let mut medidas = Vec::new();
        if self.medidas.contains(TODAS_LAS_MEDIDAS) {
            medidas.push(TODAS_LAS_MEDIDAS.into());
        }
        medidas.extend(
            cat.medidas
                .keys()
                .filter(|k| self.medidas.contains(*k))
                .cloned(),
        );
        let estados: Vec<String> = cat
            .estados
            .keys()
            .filter(|k| self.estados.contains(*k))
            .cloned()
            .collect();
        let meses: Vec<String> = cat
            .meses
            .keys()
            .filter(|k| self.meses.contains(*k))
            .cloned()
            .collect();

        Ok(SeleccionLote {
            tribunales,
            pestanas,
            modalidades,
            informes,
            medidas,
            estados,
            meses,
            anio: self.anio.clone(),
            fechas,
        })
    }

    fn plan_actual(&self) -> Result<PlanLote, String> {
        let cat = self
            .catalogo
            .as_ref()
            .ok_or_else(|| "Carga primero las opciones de la pantalla en Chrome.".to_string())?;
        planificar_lite(cat, &self.seleccion_actual(cat)?)
    }

    fn iniciar_descarga(&mut self, plan: PlanLote) {
        let Some(puente) = self.puente.clone() else {
            self.ultimo_error = Some("Conecta Chrome antes de iniciar la descarga.".into());
            return;
        };
        let Some(catalogo) = self.catalogo.clone() else {
            self.ultimo_error = Some("Carga primero las opciones de la pantalla.".into());
            return;
        };
        if self.destino.as_os_str().is_empty() {
            self.ultimo_error = Some("Elige una carpeta de destino.".into());
            return;
        }
        let destino = self.destino.clone();
        let opciones = Opciones {
            conservar_filtros: lote::conservar_filtros(self.conservar_filtros, catalogo.pantalla),
            evidencia: self.evidencia_pdf,
            pausa: Duration::from_millis(250),
        };
        self.cancelar = Arc::new(AtomicBool::new(false));
        let cancelar = self.cancelar.clone();
        self.progreso_actual = 0;
        self.progreso_total = plan.consultas.len();
        self.estado_actual = "Verificando conexión…".into();
        self.eventos.clear();
        self.ultimo_fin = None;
        self.ultimo_error = None;

        let (tx, rx) = mpsc::channel();
        self.rx_descarga = Some(rx);
        std::thread::spawn(move || {
            let res = (|| -> Result<PathBuf, String> {
                puente.verificar(EXTENSION_COMPARTIDA)?;
                let mut carpeta_creada = None;
                lote::ejecutar(
                    puente.as_ref(),
                    &catalogo,
                    EXTENSION_COMPARTIDA,
                    &plan,
                    &destino,
                    &opciones,
                    &cancelar,
                    &mut |ev| {
                        if let Evento::Carpeta(ref c) = ev {
                            carpeta_creada = Some(c.clone());
                        } else if let Evento::Fin(ref f) = ev {
                            carpeta_creada = Some(f.carpeta.clone());
                        }
                        let _ = tx.send(MensajeDescarga::Evento(ev));
                    },
                )?;
                carpeta_creada.ok_or_else(|| "La descarga terminó sin crear carpeta.".into())
            })();
            let _ = tx.send(MensajeDescarga::Terminado(res));
        });
    }

    fn retomar_pendientes(&mut self, carpeta: PathBuf) {
        let Some(cat) = self.catalogo.as_ref() else {
            self.ultimo_error =
                Some("Carga primero las opciones de la pantalla correspondiente.".into());
            return;
        };
        match lote::pendientes(&carpeta).and_then(|p| {
            if !PANTALLAS_PERMITIDAS.contains(&p.pantalla) {
                return Err("Esa carpeta no corresponde a una pantalla admitida.".into());
            }
            cat.planificar_pendientes(EXTENSION_COMPARTIDA, &p)
        }) {
            Ok(plan) => {
                self.pendientes_ruta = Some(carpeta);
                self.iniciar_descarga(plan);
            }
            Err(e) => {
                self.ultimo_error = Some(e);
            }
        }
    }

    fn agregar_carpeta_consolidar(&mut self, ruta: PathBuf) {
        if self.carpetas_consolidar.iter().any(|c| c.ruta == ruta) {
            return;
        }
        self.carpetas_consolidar.push(inspeccionar_carpeta(ruta, self.permitir_incompletos));
        self.error_consolidar = None;
    }

    fn refrescar_carpetas_consolidar(&mut self) {
        let permitir = self.permitir_incompletos;
        for c in &mut self.carpetas_consolidar {
            *c = inspeccionar_carpeta(c.ruta.clone(), permitir);
        }
    }

    fn iniciar_consolidacion(&mut self) {
        if self.carpetas_consolidar.is_empty() {
            self.error_consolidar = Some("Agrega al menos una carpeta de descarga.".into());
            return;
        }
        let rutas: Vec<PathBuf> = self
            .carpetas_consolidar
            .iter()
            .map(|c| c.ruta.clone())
            .collect();
        let permitir = self.permitir_incompletos;
        let destino = self.destino_excel.clone();
        self.error_consolidar = None;
        self.ultimo_excel = None;

        let (tx, rx) = mpsc::channel();
        self.rx_consolidar = Some(rx);
        std::thread::spawn(move || {
            let res = consolidar_carpetas(&rutas, destino.as_deref(), permitir);
            let _ = tx.send(MensajeConsolidacion::Listo(res));
        });
    }

    fn procesar_mensajes(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.rx_catalogo {
            if let Ok(MensajeCatalogo::Listo(pantalla, res)) = rx.try_recv() {
                self.rx_catalogo = None;
                match res {
                    Ok(cat) => self.aplicar_catalogo(pantalla, cat),
                    Err(e) => {
                        self.ultimo_error = Some(e);
                        self.estado_actual.clear();
                    }
                }
            } else {
                ctx.request_repaint_after(Duration::from_millis(80));
            }
        }

        if let Some(rx) = &self.rx_descarga {
            let mut termino = false;
            let mut carpeta_nueva = None;
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    MensajeDescarga::Evento(ev) => match ev {
                        Evento::Carpeta(c) => {
                            self.eventos.push(format!("Carpeta creada: {}", c.display()));
                        }
                        Evento::Plan(n) => {
                            self.progreso_total = n;
                        }
                        Evento::Consulta { indice, total, etiqueta } => {
                            self.progreso_actual = indice;
                            self.progreso_total = total;
                            self.estado_actual = format!("Consulta {indice} de {total}: {etiqueta}");
                            self.eventos.push(format!("[{indice}/{total}] {etiqueta}"));
                        }
                        Evento::Estado(s) => {
                            self.estado_actual = s;
                        }
                        Evento::Total(t) => {
                            self.eventos.push(format!("  Páginas detectadas: {t}"));
                        }
                        Evento::Pagina { pagina, total, .. } => {
                            self.estado_actual = format!("Descargando página {pagina} de {total}…");
                        }
                        Evento::Vacia { consulta } => {
                            self.eventos.push(format!("  Sin registros: {consulta}"));
                        }
                        Evento::Evidencia { archivo } => {
                            self.eventos.push(format!("  Pantallazo PDF guardado: {archivo}"));
                        }
                        Evento::ConsultaLista { consulta, estado } => {
                            self.eventos.push(format!("  {estado}: {consulta}"));
                        }
                        Evento::Aviso(a) => {
                            self.eventos.push(format!("  Aviso: {a}"));
                        }
                        Evento::Fin(fin) => {
                            self.estado_actual = format!(
                                "Descarga finalizada: {} archivo(s), {} PDF(s) sin registros, {} registro(s).",
                                fin.archivos, fin.pdfs, fin.registros
                            );
                            carpeta_nueva = Some(fin.carpeta.clone());
                            self.ultimo_fin = Some(fin);
                        }
                    },
                    MensajeDescarga::Terminado(res) => {
                        termino = true;
                        match res {
                            Ok(c) => carpeta_nueva = Some(c),
                            Err(e) => {
                                self.ultimo_error = Some(e);
                                self.estado_actual = "Descarga detenida.".into();
                            }
                        }
                    }
                }
            }
            if let Some(c) = carpeta_nueva {
                self.agregar_carpeta_consolidar(c);
            }
            if termino {
                self.rx_descarga = None;
            } else {
                ctx.request_repaint_after(Duration::from_millis(80));
            }
        }

        if let Some(rx) = &self.rx_consolidar {
            if let Ok(MensajeConsolidacion::Listo(res)) = rx.try_recv() {
                self.rx_consolidar = None;
                match res {
                    Ok(ruta) => self.ultimo_excel = Some(ruta),
                    Err(e) => self.error_consolidar = Some(e),
                }
            } else {
                ctx.request_repaint_after(Duration::from_millis(80));
            }
        }
    }

    fn dibujar(&mut self, ctx: &egui::Context) {
        if self.tema_aplicado != Some(self.oscuro) {
            theme::aplicar(ctx, self.oscuro);
            self.tema_aplicado = Some(self.oscuro);
        }
        self.procesar_mensajes(ctx);
        let p = theme::pal(self.oscuro);

        egui::SidePanel::left("nav_lite")
            .resizable(false)
            .exact_width(210.0)
            .frame(
                Frame::new()
                    .fill(p.nav)
                    .inner_margin(Margin::symmetric(14, 16)),
            )
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("Descargador")
                        .size(19.0)
                        .strong()
                        .color(Color32::WHITE),
                );
                ui.label(
                    RichText::new("SITFA · Solo lectura")
                        .size(12.0)
                        .color(p.nav_text),
                );
                ui.add_space(16.0);

                for (vista, etiqueta) in [
                    (Vista::Descargar, "Descargar"),
                    (Vista::Consolidar, "Consolidar Excel"),
                ] {
                    let activa = self.vista == vista;
                    let fill = if activa { p.primary_bg } else { Color32::TRANSPARENT };
                    let color = if activa { Color32::WHITE } else { p.nav_text };
                    let btn = egui::Button::new(RichText::new(etiqueta).color(color).strong())
                        .fill(fill)
                        .corner_radius(CornerRadius::same(7))
                        .min_size(egui::vec2(ui.available_width(), 34.0));
                    if ui.add(btn).clicked() {
                        self.vista = vista;
                    }
                    ui.add_space(4.0);
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    let etiqueta_tema = if self.oscuro { "Modo claro" } else { "Modo oscuro" };
                    if ui
                        .add(
                            egui::Button::new(RichText::new(etiqueta_tema).color(p.nav_text))
                                .fill(Color32::TRANSPARENT)
                                .stroke(Stroke::new(1.0_f32, p.border))
                                .min_size(egui::vec2(ui.available_width(), 30.0)),
                        )
                        .clicked()
                    {
                        self.oscuro = !self.oscuro;
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Solo lectura en SITFA.\nUsa tu sesión activa en Chrome.")
                            .size(11.5)
                            .color(p.nav_text),
                    );
                });
            });

        egui::CentralPanel::default()
            .frame(Frame::new().fill(p.bg).inner_margin(Margin::same(18)))
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .id_salt("scroll_principal_lite")
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.vista {
                        Vista::Descargar => self.vista_descargar(ui, &p),
                        Vista::Consolidar => self.vista_consolidar(ui, &p),
                    });
            });
    }

    fn vista_descargar(&mut self, ui: &mut Ui, p: &Pal) {
        theme::titulo(
            ui,
            p,
            "Descargar listados desde SITFA",
            "Seguimiento, Búsqueda de litigantes, Informes por vencer y Medidas por vencer.",
        );

        // Paso 1: Conexión con Chrome
        theme::card(p).show(ui, |ui| {
            theme::seccion(ui, p, "Paso 1 · Conexión con Chrome");
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if ui.button("Instalar / actualizar extensión").clicked() {
                    self.instalar_extension();
                }
                if ui.button("Abrir chrome://extensions").clicked() && !extension::abrir_extensiones_chrome() {
                    self.aviso_extension = Some((
                        "No se encontró Chrome automáticamente. Abre chrome://extensions en tu navegador.".into(),
                        true,
                    ));
                }
                ui.separator();
                if self.puente.is_none() {
                    if theme::primary(ui, p, "Iniciar conexión").clicked() {
                        self.iniciar_conexion();
                    }
                } else {
                    if ui
                        .add_enabled(!self.ocupado_descarga(), egui::Button::new("Cerrar conexión"))
                        .clicked()
                    {
                        self.cerrar_conexion();
                    }
                }
                if let Some(puente) = &self.puente {
                    let conectado = puente.conectado();
                    let (texto, color) = if conectado {
                        ("Conectado a Chrome", p.green)
                    } else {
                        ("Esperando código en Chrome", p.amber)
                    };
                    theme::chip(ui, texto, color, self.oscuro);
                } else {
                    theme::chip(ui, "Sin conexión", p.gray, self.oscuro);
                }
            });

            if let Some(puente) = &self.puente {
                let codigo = puente.codigo_conexion();
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Código de conexión:").color(p.muted));
                    ui.label(RichText::new(&codigo).monospace().strong().color(p.text));
                    if ui.button("Copiar código").clicked() {
                        ui.ctx().copy_text(codigo);
                    }
                });
            }

            if let Some((msg, es_error)) = &self.aviso_extension {
                ui.add_space(6.0);
                let color = if *es_error { p.red } else { p.green };
                ui.label(RichText::new(msg).size(12.5).color(color));
            }
        });

        ui.add_space(12.0);

        // Paso 2: Pantalla y filtros
        theme::card(p).show(ui, |ui| {
            theme::seccion(ui, p, "Paso 2 · Pantalla y filtros");
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                for pantalla in PANTALLAS_PERMITIDAS {
                    if ui
                        .add_enabled(
                            !self.ocupado_descarga() && !self.ocupado_catalogo(),
                            egui::Button::new(pantalla.etiqueta()).selected(self.pantalla == pantalla),
                        )
                        .clicked()
                        && self.pantalla != pantalla
                    {
                        self.pantalla = pantalla;
                        self.catalogo = None;
                    }
                }
                ui.separator();
                let conectado = self.puente.as_ref().is_some_and(|b| b.conectado());
                let puede_cargar = conectado && !self.ocupado_catalogo() && !self.ocupado_descarga();
                if ui
                    .add_enabled(puede_cargar, egui::Button::new("Cargar opciones de SITFA"))
                    .clicked()
                {
                    self.cargar_catalogo();
                }
                if self.ocupado_catalogo() {
                    ui.spinner();
                }
            });

            if let Some(cat) = self.catalogo.clone() {
                ui.add_space(10.0);
                self.dibujar_filtros(ui, p, &cat);
            } else {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        "Conecta Chrome con tu sesión de SITFA y pulsa «Cargar opciones de SITFA» para ver tribunales, pestañas y filtros disponibles.",
                    )
                    .size(13.0)
                    .color(p.muted),
                );
            }
        });

        ui.add_space(12.0);

        // Paso 3: Destino, evidencia PDF y ejecución
        theme::card(p).show(ui, |ui| {
            theme::seccion(ui, p, "Paso 3 · Descarga");
            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Carpeta de destino:").color(p.muted));
                ui.label(
                    RichText::new(self.destino.display().to_string())
                        .monospace()
                        .size(12.5),
                );
                if ui
                    .add_enabled(!self.ocupado_descarga(), egui::Button::new("Elegir carpeta…"))
                    .clicked()
                {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        self.destino = dir;
                    }
                }
            });

            ui.add_space(6.0);
            ui.add_enabled(
                !self.ocupado_descarga(),
                egui::Checkbox::new(
                    &mut self.evidencia_pdf,
                    "Guardar pantallazo en PDF cuando no existen registros",
                ),
            );

            ui.add_space(8.0);
            let plan_res = self.plan_actual();
            match &plan_res {
                Ok(plan) => {
                    theme::chip(
                        ui,
                        &format!("{} consulta(s) listas para descargar", plan.consultas.len()),
                        p.green,
                        self.oscuro,
                    );
                }
                Err(aviso) => {
                    ui.label(RichText::new(aviso).size(12.5).color(p.amber));
                }
            }

            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                let conectado = self.puente.as_ref().is_some_and(|b| b.conectado());
                let puede_iniciar = conectado && !self.ocupado_descarga() && plan_res.is_ok();
                if ui
                    .add_enabled(
                        puede_iniciar,
                        egui::Button::new(RichText::new("Iniciar descarga").color(Color32::WHITE).strong())
                            .fill(p.primary_bg)
                            .corner_radius(CornerRadius::same(7)),
                    )
                    .clicked()
                {
                    if let Ok(plan) = plan_res {
                        self.iniciar_descarga(plan);
                    }
                }

                let puede_retomar =
                    conectado && !self.ocupado_descarga() && self.catalogo.is_some();
                if ui
                    .add_enabled(puede_retomar, egui::Button::new("Retomar pendientes…"))
                    .clicked()
                {
                    if let Some(carpeta) = rfd::FileDialog::new().pick_folder() {
                        self.retomar_pendientes(carpeta);
                    }
                }

                if self.ocupado_descarga()
                    && ui.button("Cancelar descarga").clicked()
                {
                    self.cancelar.store(true, Ordering::SeqCst);
                    self.estado_actual = "Cancelando descarga…".into();
                }
            });

            if self.progreso_total > 0 {
                ui.add_space(8.0);
                let fraccion = (self.progreso_actual as f32 / self.progreso_total as f32).clamp(0.0, 1.0);
                ui.add(
                    egui::ProgressBar::new(fraccion)
                        .text(format!("{} / {}", self.progreso_actual, self.progreso_total)),
                );
            }
            if !self.estado_actual.is_empty() {
                ui.add_space(4.0);
                ui.label(RichText::new(&self.estado_actual).size(13.0).color(p.text));
            }
            if let Some(err) = &self.ultimo_error {
                ui.add_space(4.0);
                ui.label(RichText::new(err).size(13.0).color(p.red));
            }
            if let Some(fin) = &self.ultimo_fin {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Abrir carpeta descargada").clicked() {
                        abrir_en_explorador(&fin.carpeta);
                    }
                    if ui.button("Ir a Consolidar Excel").clicked() {
                        self.vista = Vista::Consolidar;
                    }
                });
            }

            if !self.eventos.is_empty() {
                ui.add_space(8.0);
                Frame::new()
                    .fill(p.bg)
                    .stroke(Stroke::new(1.0_f32, p.border))
                    .corner_radius(CornerRadius::same(6))
                    .inner_margin(Margin::same(8))
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .id_salt("eventos_descarga_lite")
                            .max_height(150.0)
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                for linea in &self.eventos {
                                    ui.label(RichText::new(linea).monospace().size(12.0));
                                }
                            });
                    });
            }
        });
    }

    fn dibujar_filtros(&mut self, ui: &mut Ui, p: &Pal, cat: &Catalogo) {
        let habilitado = !self.ocupado_descarga();

        // Tribunales
        ui.horizontal(|ui| {
            ui.label(RichText::new("Tribunales:").strong());
            ui.add_enabled(
                habilitado,
                egui::TextEdit::singleline(&mut self.busqueda_tribunal)
                    .hint_text("Filtrar tribunal…")
                    .desired_width(220.0),
            );
            if ui.add_enabled(habilitado, egui::Button::new("Todos")).clicked() {
                self.tribunales = cat.tribunales.keys().cloned().collect();
            }
            if ui.add_enabled(habilitado, egui::Button::new("Ninguno")).clicked() {
                self.tribunales.clear();
            }
            ui.label(
                RichText::new(format!("{}/{}", self.tribunales.len(), cat.tribunales.len()))
                    .size(12.0)
                    .color(p.muted),
            );
        });
        let filtro = self.busqueda_tribunal.trim().to_lowercase();
        Frame::new()
            .fill(p.bg)
            .stroke(Stroke::new(1.0_f32, p.border))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(Margin::same(8))
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .id_salt("lista_tribunales_lite")
                    .max_height(140.0)
                    .show(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            for (clave, nombre) in &cat.tribunales {
                                if !filtro.is_empty() && !nombre.to_lowercase().contains(&filtro) {
                                    continue;
                                }
                                let mut marcado = self.tribunales.contains(clave);
                                if ui
                                    .add_enabled(habilitado, egui::Checkbox::new(&mut marcado, nombre))
                                    .changed()
                                {
                                    if marcado {
                                        self.tribunales.insert(clave.clone());
                                    } else {
                                        self.tribunales.remove(clave);
                                    }
                                }
                            }
                        });
                    });
            });

        ui.add_space(8.0);

        match cat.pantalla {
            Pantalla::Seguimiento => {
                // Pestañas
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Pestañas:").strong());
                    for pestana in PESTANAS {
                        if !cat.pestanas.iter().any(|x| x == pestana) {
                            continue;
                        }
                        let mut marcado = self.pestanas.contains(pestana);
                        if ui
                            .add_enabled(habilitado, egui::Checkbox::new(&mut marcado, pestana))
                            .changed()
                        {
                            if marcado {
                                self.pestanas.insert(pestana.into());
                            } else {
                                self.pestanas.remove(pestana);
                            }
                        }
                    }
                    if ui.add_enabled(habilitado, egui::Button::new("Todas")).clicked() {
                        self.pestanas = cat.pestanas.iter().cloned().collect();
                    }
                    if ui.add_enabled(habilitado, egui::Button::new("Ninguna")).clicked() {
                        self.pestanas.clear();
                    }
                });

                // Modalidades
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Modalidades:").strong());
                    for codigo in MODALIDADES {
                        let Some(nombre) = cat.modalidades.get(codigo) else {
                            continue;
                        };
                        let mut marcado = self.modalidades.contains(codigo);
                        if ui
                            .add_enabled(habilitado, egui::Checkbox::new(&mut marcado, nombre))
                            .changed()
                        {
                            if marcado {
                                self.modalidades.insert(codigo.into());
                            } else {
                                self.modalidades.remove(codigo);
                            }
                        }
                    }
                    if ui.add_enabled(habilitado, egui::Button::new("Todas")).clicked() {
                        self.modalidades = cat.modalidades.keys().cloned().collect();
                    }
                    if ui.add_enabled(habilitado, egui::Button::new("Ninguna")).clicked() {
                        self.modalidades.clear();
                    }
                });

                // Filtros propios de Cumplimiento
                if self.pestanas.contains("Cumplimiento") {
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Filtro de medidas (Cumplimiento):").strong());
                        let mut todas = self.medidas.contains(TODAS_LAS_MEDIDAS);
                        if ui
                            .add_enabled(
                                habilitado,
                                egui::Checkbox::new(&mut todas, "Todas las medidas"),
                            )
                            .changed()
                        {
                            if todas {
                                self.medidas.insert(TODAS_LAS_MEDIDAS.into());
                            } else {
                                self.medidas.remove(TODAS_LAS_MEDIDAS);
                            }
                        }
                        for (clave, nombre) in &cat.medidas {
                            let mut marcado = self.medidas.contains(clave);
                            if ui
                                .add_enabled(habilitado, egui::Checkbox::new(&mut marcado, nombre))
                                .changed()
                            {
                                if marcado {
                                    self.medidas.insert(clave.clone());
                                } else {
                                    self.medidas.remove(clave);
                                }
                            }
                        }
                    });
                }

                // Filtros propios de Informes
                if self.pestanas.contains("Informes") {
                    ui.add_space(4.0);
                    dibujar_grupo_mapa(
                        ui,
                        habilitado,
                        "Tipos de informe (Informes):",
                        &cat.informes,
                        &mut self.informes,
                    );
                }
            }
            Pantalla::Litigantes => {
                dibujar_grupo_mapa(ui, habilitado, "Estados:", &cat.estados, &mut self.estados);
            }
            Pantalla::CalendarioInformes | Pantalla::CalendarioMedidas => {
                dibujar_grupo_mapa(ui, habilitado, "Meses:", &cat.meses, &mut self.meses);
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Año:").strong());
                    egui::ComboBox::from_id_salt("anio_lite")
                        .selected_text(cat.anios.get(&self.anio).map(String::as_str).unwrap_or("Elegir"))
                        .show_ui(ui, |ui| {
                            for (clave, nombre) in &cat.anios {
                                ui.selectable_value(&mut self.anio, clave.clone(), nombre);
                            }
                        });
                });
            }
            Pantalla::Carga => {}
        }

        // Fechas y conservación de filtros (solo Seguimiento y Litigantes)
        if usa_fechas(cat.pantalla) {
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Fechas:").strong());
                ui.add_enabled(
                    habilitado,
                    egui::Checkbox::new(&mut self.todas_las_fechas, "Todas las fechas"),
                );
                if !self.todas_las_fechas {
                    ui.label("Desde:");
                    ui.add_enabled(
                        habilitado,
                        egui::TextEdit::singleline(&mut self.desde_texto)
                            .hint_text("dd/mm/aaaa")
                            .desired_width(105.0),
                    );
                    ui.label("Hasta:");
                    ui.add_enabled(
                        habilitado,
                        egui::TextEdit::singleline(&mut self.hasta_texto)
                            .hint_text("dd/mm/aaaa")
                            .desired_width(105.0),
                    );
                }
            });
            ui.add_enabled(
                habilitado,
                egui::Checkbox::new(
                    &mut self.conservar_filtros,
                    "Conservar filtros adicionales activos en el formulario de SITFA",
                ),
            );
        }
    }

    fn vista_consolidar(&mut self, ui: &mut Ui, p: &Pal) {
        theme::titulo(
            ui,
            p,
            "Consolidar descargas en Excel",
            "Reúne una o más carpetas descargadas en un único libro Excel (.xlsx) con trazabilidad por archivo.",
        );

        theme::card(p).show(ui, |ui| {
            theme::seccion(ui, p, "Carpetas de descarga");
            ui.add_space(6.0);

            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(!self.ocupado_consolidar(), egui::Button::new("Agregar carpeta…"))
                    .clicked()
                {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        self.agregar_carpeta_consolidar(dir);
                    }
                }
                if ui
                    .add_enabled(
                        !self.ocupado_consolidar() && !self.carpetas_consolidar.is_empty(),
                        egui::Button::new("Limpiar lista"),
                    )
                    .clicked()
                {
                    self.carpetas_consolidar.clear();
                    self.ultimo_excel = None;
                    self.error_consolidar = None;
                }
                if ui
                    .add_enabled(
                        !self.ocupado_consolidar(),
                        egui::Checkbox::new(
                            &mut self.permitir_incompletos,
                            "Incluir descargas incompletas",
                        ),
                    )
                    .changed()
                {
                    self.refrescar_carpetas_consolidar();
                }
            });

            ui.add_space(8.0);
            if self.carpetas_consolidar.is_empty() {
                ui.label(
                    RichText::new("Todavía no hay carpetas agregadas. Al terminar una descarga se agrega aquí automáticamente, o puedes agregarlas manualmente.")
                        .size(13.0)
                        .color(p.muted),
                );
            } else {
                let mut quitar = None;
                for (i, item) in self.carpetas_consolidar.iter().enumerate() {
                    Frame::new()
                        .fill(p.bg)
                        .stroke(Stroke::new(1.0_f32, p.border))
                        .corner_radius(CornerRadius::same(6))
                        .inner_margin(Margin::same(8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        RichText::new(item.ruta.display().to_string())
                                            .monospace()
                                            .size(12.5),
                                    );
                                    let color = if !item.valida {
                                        p.red
                                    } else if item.parcial {
                                        p.amber
                                    } else {
                                        p.green
                                    };
                                    ui.label(RichText::new(&item.resumen).size(12.0).color(color));
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui
                                        .add_enabled(!self.ocupado_consolidar(), egui::Button::new("Quitar"))
                                        .clicked()
                                    {
                                        quitar = Some(i);
                                    }
                                });
                            });
                        });
                    ui.add_space(4.0);
                }
                if let Some(i) = quitar {
                    self.carpetas_consolidar.remove(i);
                }
            }

            ui.add_space(10.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("Carpeta de salida del Excel:").color(p.muted));
                let etiqueta_salida = self
                    .destino_excel
                    .as_ref()
                    .map(|d| d.display().to_string())
                    .unwrap_or_else(|| "Misma carpeta de la primera descarga".into());
                ui.label(RichText::new(etiqueta_salida).monospace().size(12.5));
                if ui
                    .add_enabled(!self.ocupado_consolidar(), egui::Button::new("Elegir carpeta…"))
                    .clicked()
                {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        self.destino_excel = Some(dir);
                    }
                }
                if self.destino_excel.is_some()
                    && ui
                        .add_enabled(!self.ocupado_consolidar(), egui::Button::new("Por defecto"))
                        .clicked()
                {
                    self.destino_excel = None;
                }
            });

            ui.add_space(10.0);
            let listas = !self.carpetas_consolidar.is_empty()
                && self.carpetas_consolidar.iter().all(|c| c.valida);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        listas && !self.ocupado_consolidar(),
                        egui::Button::new(
                            RichText::new("Consolidar en Excel (.xlsx)")
                                .color(Color32::WHITE)
                                .strong(),
                        )
                        .fill(p.primary_bg)
                        .corner_radius(CornerRadius::same(7)),
                    )
                    .clicked()
                {
                    self.iniciar_consolidacion();
                }
                if self.ocupado_consolidar() {
                    ui.spinner();
                    ui.label("Generando libro Excel…");
                }
            });

            if let Some(err) = &self.error_consolidar {
                ui.add_space(6.0);
                ui.label(RichText::new(err).size(13.0).color(p.red));
            }
            if let Some(ruta) = &self.ultimo_excel {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("Excel consolidado guardado: {}", ruta.display()))
                        .size(13.0)
                        .color(p.green),
                );
                if let Some(dir) = ruta.parent() {
                    if ui.button("Abrir carpeta del Excel").clicked() {
                        abrir_en_explorador(dir);
                    }
                }
            }
        });
    }
}

impl eframe::App for LiteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.dibujar(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.cancelar.store(true, Ordering::SeqCst);
        self.cerrar_conexion();
    }
}

fn dibujar_grupo_mapa(
    ui: &mut Ui,
    habilitado: bool,
    titulo: &str,
    opciones: &BTreeMap<String, String>,
    seleccion: &mut BTreeSet<String>,
) {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(titulo).strong());
        for (clave, nombre) in opciones {
            let mut marcado = seleccion.contains(clave);
            if ui
                .add_enabled(habilitado, egui::Checkbox::new(&mut marcado, nombre))
                .changed()
            {
                if marcado {
                    seleccion.insert(clave.clone());
                } else {
                    seleccion.remove(clave);
                }
            }
        }
        if ui.add_enabled(habilitado, egui::Button::new("Todos")).clicked() {
            *seleccion = opciones.keys().cloned().collect();
        }
        if ui.add_enabled(habilitado, egui::Button::new("Ninguno")).clicked() {
            seleccion.clear();
        }
    });
}

fn usa_fechas(pantalla: Pantalla) -> bool {
    matches!(pantalla, Pantalla::Seguimiento | Pantalla::Litigantes)
}

fn parsear_fecha(texto: &str) -> Option<NaiveDate> {
    let t = texto.trim();
    ["%d/%m/%Y", "%d-%m-%Y", "%Y-%m-%d"]
        .iter()
        .find_map(|fmt| NaiveDate::parse_from_str(t, fmt).ok())
}

fn planificar_lite(cat: &Catalogo, seleccion: &SeleccionLote) -> Result<PlanLote, String> {
    if !PANTALLAS_PERMITIDAS.contains(&cat.pantalla) {
        return Err("Esta pantalla no está disponible en Descargador SITFA Lite.".into());
    }
    cat.planificar(EXTENSION_COMPARTIDA, seleccion)
}

fn inspeccionar_carpeta(ruta: PathBuf, permitir_incompletos: bool) -> CarpetaConsolidacion {
    match paquete::cargar_lote_con(&ruta, permitir_incompletos) {
        Ok(lote) => {
            let estado = if lote.parcial { "Incompleta" } else { "Verificada" };
            let resumen = format!("{estado} · {} consulta(s)", lote.consultas.len());
            CarpetaConsolidacion {
                ruta,
                resumen,
                parcial: lote.parcial,
                valida: true,
            }
        }
        Err(e) => CarpetaConsolidacion {
            ruta,
            resumen: e,
            parcial: false,
            valida: false,
        },
    }
}

fn consolidar_carpetas(
    rutas: &[PathBuf],
    destino: Option<&Path>,
    permitir_incompletos: bool,
) -> Result<PathBuf, String> {
    let mut lotes: Vec<Lote> = Vec::with_capacity(rutas.len());
    for ruta in rutas {
        lotes.push(paquete::cargar_lote_con(ruta, permitir_incompletos)?);
    }
    let ahora = Local::now().fixed_offset();
    resultados::exportar_libro(&lotes, destino, false, &ahora)
}

fn carpeta_descargas_por_defecto() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(|h| PathBuf::from(h).join("Downloads"))
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir)
}

fn abrir_en_explorador(ruta: &Path) {
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(ruta).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(ruta).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(ruta).spawn();
}

fn main() -> eframe::Result<()> {
    let opciones = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(TITULO_VENTANA)
            .with_inner_size(TAMANO_INICIAL)
            .with_min_inner_size(TAMANO_MINIMO),
        ..Default::default()
    };
    eframe::run_native(
        TITULO_VENTANA,
        opciones,
        Box::new(|_cc| Ok(Box::new(LiteApp::nueva()))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use flow_rust::descargador::lote::Conexion;
    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};
    use std::io::Cursor;
    use std::sync::Mutex;

    fn catalogo_demo(pantalla: Pantalla) -> Catalogo {
        serde_json::from_value(json!({
            "pantalla": pantalla,
            "tribunales": {"10": "Juzgado de Familia Ficticio"},
            "pestanas": ["Espera", "Cumplimiento", "Informes", "Egresados"],
            "modalidades": {"1": "Residencia", "2": "Ambulatorio", "3": "FAE", "4": "DCE"},
            "informes": {"1": "Pendientes", "2": "Por vencer"},
            "medidas": {"1": "Vigentes", "2": "Vencidas"},
            "estados": {"1": "Vigentes", "2": "Egresados"},
            "meses": {"10": "Octubre", "11": "Noviembre"},
            "anios": {"2026": "2026"}
        }))
        .unwrap()
    }

    #[test]
    fn solo_admite_las_cuatro_pantallas_permitidas() {
        assert_eq!(
            PANTALLAS_PERMITIDAS,
            [
                Pantalla::Seguimiento,
                Pantalla::Litigantes,
                Pantalla::CalendarioInformes,
                Pantalla::CalendarioMedidas,
            ]
        );
        let cat_carga = catalogo_demo(Pantalla::Carga);
        let sel = SeleccionLote {
            tribunales: vec!["10".into()],
            fechas: Some((
                NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
                NaiveDate::from_ymd_opt(2026, 10, 15).unwrap(),
            )),
            ..Default::default()
        };
        assert!(planificar_lite(&cat_carga, &sel).is_err());
    }

    #[test]
    fn usa_la_misma_extension_que_la_version_completa() {
        assert_eq!(EXTENSION_COMPARTIDA, Edicion::Integral);
        assert_eq!(extension::version(EXTENSION_COMPARTIDA), "2.10.0");
    }

    #[test]
    fn planifica_las_cuatro_pantallas_con_sus_filtros_y_fechas() {
        let desde = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let hasta = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();

        let seg = planificar_lite(
            &catalogo_demo(Pantalla::Seguimiento),
            &SeleccionLote {
                tribunales: vec!["10".into()],
                pestanas: vec!["Espera".into(), "Cumplimiento".into(), "Informes".into(), "Egresados".into()],
                modalidades: vec!["1".into(), "2".into()],
                informes: vec!["1".into()],
                medidas: vec![TODAS_LAS_MEDIDAS.into(), "1".into()],
                fechas: Some((desde, hasta)),
                ..Default::default()
            },
        )
        .unwrap();
        // Espera(2) + Cumplimiento(2*2) + Informes(2*1) + Egresados(2) = 10 consultas
        assert_eq!(seg.consultas.len(), 10);
        assert_eq!(seg.fechas, Some((desde, hasta)));

        let lit = planificar_lite(
            &catalogo_demo(Pantalla::Litigantes),
            &SeleccionLote {
                tribunales: vec!["10".into()],
                estados: vec!["1".into(), "2".into()],
                fechas: Some((desde, hasta)),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(lit.consultas.len(), 2);

        for cal in [Pantalla::CalendarioInformes, Pantalla::CalendarioMedidas] {
            let plan = planificar_lite(
                &catalogo_demo(cal),
                &SeleccionLote {
                    tribunales: vec!["10".into()],
                    meses: vec!["10".into(), "11".into()],
                    anio: "2026".into(),
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(plan.consultas.len(), 2);
        }
    }

    #[test]
    fn parsear_fecha_valida_formatos_y_rechaza_invalidas() {
        assert_eq!(
            parsear_fecha("01/11/2026"),
            NaiveDate::from_ymd_opt(2026, 11, 1)
        );
        assert_eq!(
            parsear_fecha("26-11-2026"),
            NaiveDate::from_ymd_opt(2026, 11, 26)
        );
        assert_eq!(
            parsear_fecha("2026-11-26"),
            NaiveDate::from_ymd_opt(2026, 11, 26)
        );
        assert!(parsear_fecha("32/13/2026").is_none());
    }

    #[test]
    fn no_menciona_funcionalidades_no_incorporadas_en_la_interfaz() {
        let codigo = include_str!("descargador_lite.rs");
        // Revisar solo la sección de aplicación (antes de #[cfg(test)]).
        let parte_ui = codigo.split("#[cfg(test)]").next().unwrap_or(codigo);
        // Extraer literales visibles entre comillas en llamadas de UI.
        let prohibidos = [
            "Asistente",
            "Meta 4",
            "Anonimizador",
            "Bitácora",
            "Bitacora",
            "Paquete CSMP",
            "Descarga conjunta",
            "Libro CSMP",
            "Outlook",
            "Resoluciones",
            "Correos",
            "Flow Python",
            "Modo Rápido",
            "Histórico",
            "Escritos",
        ];
        for termino in prohibidos {
            assert!(
                !parte_ui.contains(termino),
                "La interfaz de descargador_lite no debe mencionar «{termino}»"
            );
        }
    }

    struct PuenteFicticio {
        ultimo_html_evidencia: Mutex<String>,
    }

    fn png_valido() -> String {
        let mut bytes = Vec::new();
        image::RgbImage::from_pixel(800, 400, image::Rgb([245, 247, 250]))
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        STANDARD.encode(bytes)
    }

    fn html_con_registros(pares: &[(String, String)], rit: &str, nombre: &str) -> String {
        let inputs: String = pares
            .iter()
            .map(|(k, v)| format!(r#"<input type="hidden" name="{k}" value="{v}">"#))
            .collect();
        format!(
            r#"<html><body>
            <script>function ShowExcel() {{ window.open("https://familia.pjud.cl/sitfa/reportes/100_Espera.xls"); }}</script>
            <form name="InformesPpalForm" action="/SITFAWEB/InformesDAction.do" method="post">{inputs}
              <input type="hidden" name="NUM_PaginaEspera" value="1">
              <input type="hidden" name="NUM_TotalEspera" value="1">
              <table id="tablaEspera">
                <tr><th>RIT</th><th>NOMBRE</th><th>TRIBUNAL</th><th>PROGRAMA</th><th>T ESPERA</th></tr>
                <tr><td>{rit}</td><td>{nombre}</td><td>Juzgado de Familia Ficticio</td><td>FAE Ficticio</td><td>15</td></tr>
              </table>
            </form>
            </body></html>"#
        )
    }

    fn html_sin_registros(pares: &[(String, String)], contexto: Option<(&str, &str)>) -> String {
        let inputs: String = pares
            .iter()
            .map(|(k, v)| format!(r#"<input type="hidden" name="{k}" value="{v}">"#))
            .collect();
        let banner = contexto
            .map(|(c, json_ctx)| {
                let esc = json_ctx.replace('&', "&amp;").replace('"', "&quot;");
                format!(r#"<div id="sitfaContextoConsulta" data-context="{esc}">{c}</div>"#)
            })
            .unwrap_or_default();
        format!(
            r#"<html><body>{banner}
            <form name="InformesPpalForm" action="/SITFAWEB/InformesDAction.do" method="post">{inputs}
              <table id="tablaEspera">
                <tr><th>RIT</th><th>NOMBRE</th><th>TRIBUNAL</th><th>PROGRAMA</th><th>T ESPERA</th></tr>
              </table>
            </form>
            <p>No existen registros para la consulta realizada.</p>
            </body></html>"#
        )
    }

    fn xls_html_ficticio(rit: &str, nombre: &str) -> String {
        format!(
            r#"<html><body><table>
            <tr><th>RIT</th><th>NOMBRE</th><th>TRIBUNAL</th><th>PROGRAMA</th><th>T ESPERA</th></tr>
            <tr><td>{rit}</td><td>{nombre}</td><td>Juzgado de Familia Ficticio</td><td>FAE Ficticio</td><td>15</td></tr>
            </table></body></html>"#
        )
    }

    impl Conexion for PuenteFicticio {
        fn llamar(&self, comando: &str, payload: Value, _plazo: Duration) -> Result<Value, String> {
            match comando {
                "lock" | "unlock" | "evidence_close" => Ok(json!({"ok": true})),
                "prepare" => {
                    let sel = &payload["selection"];
                    let mod_code = sel["modality"].as_str().unwrap_or("2");
                    Ok(json!({
                        "action": "/SITFAWEB/InformesDAction.do",
                        "pairs": [
                            ["COD_Tribunal_sel", "10"],
                            ["TIP_Consulta", mod_code],
                            ["COD_Lengueta", "tdEspera"],
                            ["FLG_Consulta", "0"],
                            ["irAccion", "Buscar medida"]
                        ]
                    }))
                }
                "search" => {
                    let pares: Vec<(String, String)> = payload["pairs"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|p| (p[0].as_str().unwrap().into(), p[1].as_str().unwrap().into()))
                        .collect();
                    let es_vacia = pares.iter().any(|(k, v)| k == "TIP_Consulta" && v == "1");
                    let html = if es_vacia {
                        html_sin_registros(&pares, None)
                    } else {
                        html_con_registros(&pares, "P-1-2026", "Persona Ficticia")
                    };
                    Ok(json!({"status": 200, "type": "text/html", "body": STANDARD.encode(html.as_bytes())}))
                }
                "download" => {
                    let xls = xls_html_ficticio("P-1-2026", "Persona Ficticia");
                    Ok(json!({"status": 200, "type": "application/vnd.ms-excel", "body": STANDARD.encode(xls.as_bytes())}))
                }
                "evidence_open" => {
                    let pares: Vec<(String, String)> = payload["pairs"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|p| (p[0].as_str().unwrap().into(), p[1].as_str().unwrap().into()))
                        .collect();
                    let partes_ctx: Vec<String> = payload["context"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|p| {
                            Some(format!("{}: {}", p[0].as_str()?, p[1].as_str()?))
                        })
                        .collect();
                    let ctx = format!("Consulta solicitada · {}", partes_ctx.join(" · "));
                    let json_ctx = serde_json::to_string(&payload["context"]).unwrap();
                    let html = html_sin_registros(&pares, Some((&ctx, &json_ctx)));
                    let digest = format!("{:x}", Sha256::digest(html.as_bytes()));
                    *self.ultimo_html_evidencia.lock().unwrap() = digest.clone();
                    Ok(json!({"status": 200, "type": "text/html", "body": STANDARD.encode(html.as_bytes()), "digest": digest}))
                }
                "evidence_capture" => {
                    let digest = self.ultimo_html_evidencia.lock().unwrap().clone();
                    Ok(json!({"png": png_valido(), "digest": digest}))
                }
                otro => Err(format!("Comando inesperado: {otro}")),
            }
        }
    }

    #[test]
    fn descarga_etiqueta_archivos_genera_pdf_sin_registros_y_consolida_excel() {
        let mut sal = [0u8; 8];
        getrandom::fill(&mut sal).unwrap();
        let base = std::env::temp_dir().join(format!(
            "descargador-lite-test-{}",
            sal.iter().map(|b| format!("{b:02x}")).collect::<String>()
        ));
        std::fs::create_dir_all(&base).unwrap();

        let cat = catalogo_demo(Pantalla::Seguimiento);
        let plan = planificar_lite(
            &cat,
            &SeleccionLote {
                tribunales: vec!["10".into()],
                pestanas: vec!["Espera".into()],
                // Modalidad 1 devolverá 0 registros (PDF), Modalidad 2 devolverá 1 registro (XLS).
                modalidades: vec!["1".into(), "2".into()],
                ..Default::default()
            },
        )
        .unwrap();

        let puente = PuenteFicticio {
            ultimo_html_evidencia: Mutex::new(String::new()),
        };
        let cancelar = AtomicBool::new(false);
        let mut fin = None;
        lote::ejecutar(
            &puente,
            &cat,
            EXTENSION_COMPARTIDA,
            &plan,
            &base,
            &Opciones {
                conservar_filtros: false,
                evidencia: true,
                pausa: Duration::ZERO,
            },
            &cancelar,
            &mut |ev| {
                if let Evento::Fin(f) = ev {
                    fin = Some(f);
                }
            },
        )
        .unwrap();

        let fin = fin.expect("debe emitir Evento::Fin");
        assert_eq!(fin.consultas, 2);
        assert_eq!(fin.archivos, 1);
        assert_eq!(fin.pdfs, 1);
        assert_eq!(fin.registros, 1);

        // Verificar etiquetado oficial de XLS y PDF.
        let archivos: Vec<String> = std::fs::read_dir(&fin.carpeta)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            archivos.iter().any(|n| n.starts_with("ESP Residencia Jdo Familia Ficticio") && n.ends_with(".pdf")),
            "falta PDF etiquetado para consulta sin registros: {archivos:?}"
        );
        assert!(
            archivos.iter().any(|n| n.starts_with("ESP Ambulatorio Jdo Familia Ficticio") && n.ends_with(".xls")),
            "falta XLS etiquetado para consulta con registros: {archivos:?}"
        );

        // Consolidar la descarga en un libro Excel (.xlsx).
        let excel = consolidar_carpetas(std::slice::from_ref(&fin.carpeta), Some(&base), false).unwrap();
        assert!(excel.is_file());
        assert_eq!(excel.extension().and_then(|e| e.to_str()), Some("xlsx"));
        assert!(excel.with_extension("json").is_file());
        assert!(std::fs::metadata(&excel).unwrap().len() > 500);

        let _ = std::fs::remove_dir_all(&base);
    }
}
