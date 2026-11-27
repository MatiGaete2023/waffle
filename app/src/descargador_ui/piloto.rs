use crate::theme::Pal;
use egui::{RichText, Ui};
use flow_rust::descargador::{lote, multiventana, piloto, PlanLote, Puente};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::time::Duration;

enum Mensaje {
    Avance(piloto::Evento),
    Termino(Result<piloto::Medidas, String>),
    Comparacion(Result<piloto::Comparacion, String>),
}

#[derive(Default)]
pub struct Piloto {
    adicionales: [Option<Arc<Puente>>; 2],
    usar_tres: bool,
    recepcion: Option<mpsc::Receiver<Mensaje>>,
    cancelar: Option<Arc<AtomicBool>>,
    avance: [String; piloto::PESTANAS],
    error: Option<String>,
    carpeta: Option<PathBuf>,
    referencia: Option<PathBuf>,
    medidas: Option<piloto::Medidas>,
    comparacion: Option<piloto::Comparacion>,
}

impl Piloto {
    pub fn activo(&self) -> bool {
        self.recepcion.is_some()
    }

    pub fn recibir(&mut self) {
        let Some(rx) = &self.recepcion else { return };
        loop {
            match rx.try_recv() {
                Ok(Mensaje::Avance(piloto::Evento::Carpeta(ruta))) => self.carpeta = Some(ruta),
                Ok(Mensaje::Avance(piloto::Evento::Avance { pestana, texto })) => {
                    if let Some(valor) = self.avance.get_mut(pestana) {
                        *valor = texto;
                    }
                }
                Ok(Mensaje::Termino(resultado)) => {
                    match resultado {
                        Ok(m) => {
                            if m.pestanas == 1 {
                                self.referencia = self.carpeta.clone();
                            }
                            self.medidas = Some(m);
                        }
                        Err(error) => self.error = Some(error),
                    }
                    break;
                }
                Ok(Mensaje::Comparacion(resultado)) => {
                    match resultado {
                        Ok(c) => self.comparacion = Some(c),
                        Err(error) => self.error = Some(error),
                    }
                    break;
                }
                Err(mpsc::TryRecvError::Empty) => return,
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.error =
                        Some("Se interrumpió el piloto. Revisa la carpeta de la medición.".into());
                    break;
                }
            }
        }
        self.recepcion = None;
        self.cancelar = None;
    }

    fn iniciar(
        &mut self,
        ui: &Ui,
        puentes: Vec<Arc<Puente>>,
        plan: PlanLote,
        destino: PathBuf,
        pausa: Duration,
    ) {
        let (tx, rx) = mpsc::channel();
        let cancelar = Arc::new(AtomicBool::new(false));
        self.recepcion = Some(rx);
        self.cancelar = Some(cancelar.clone());
        self.avance = Default::default();
        self.error = None;
        self.carpeta = None;
        self.medidas = None;
        self.comparacion = None;
        let ctx = ui.ctx().clone();
        std::thread::spawn(move || {
            let resultado =
                piloto::ejecutar(&puentes, &plan, &destino, pausa, &cancelar, &|evento| {
                    let _ = tx.send(Mensaje::Avance(evento));
                    ctx.request_repaint();
                });
            let _ = tx.send(Mensaje::Termino(resultado));
            ctx.request_repaint();
        });
    }

    fn comparar(&mut self, ui: &Ui, una: PathBuf, tres: PathBuf) {
        let (tx, rx) = mpsc::channel();
        self.recepcion = Some(rx);
        self.error = None;
        self.comparacion = None;
        let ctx = ui.ctx().clone();
        std::thread::spawn(move || {
            let _ = tx.send(Mensaje::Comparacion(piloto::comparar(&una, &tres)));
            ctx.request_repaint();
        });
    }

    pub fn conexion(&self, principal: Arc<Puente>) -> Arc<dyn lote::Conexion + Send + Sync> {
        match (&self.adicionales, self.usar_tres) {
            ([Some(dos), Some(tres)], true) => Arc::new(multiventana::Conexion {
                principal,
                adicionales: [dos.clone(), tres.clone()],
            }),
            _ => principal,
        }
    }

    pub fn mostrar_conexiones(&mut self, ui: &mut Ui, p: &Pal) {
        let actual = self
            .adicionales
            .iter()
            .flatten()
            .filter(|p| p.conectado())
            .count();
        egui::CollapsingHeader::new("Descarga con tres pestañas")
            .default_open(self.usar_tres)
            .show(ui, |ui| {
                ui.checkbox(&mut self.usar_tres, "Usar tres pestañas en las descargas");
                ui.label("La pestaña 1 usa la conexión principal. Abre SITFA en otras dos pestañas y pega un código distinto en cada una. Extensión Integral 2.11.1 o posterior.");
                ui.label("Los listados se reparten entre las conexiones. Las bitácoras, historia e histórico usan la sesión principal. Con PDF o filtros del formulario se usa la pestaña principal.");
                ui.ctx().request_repaint_after(Duration::from_millis(500));
                for (i, extra) in self.adicionales.iter_mut().enumerate() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("Pestaña {}", i + 2));
                        if let Some(puente) = extra {
                            if ui.button(format!("Copiar código {}", i + 2)).clicked() {
                                ui.ctx().copy_text(puente.codigo_conexion());
                            }
                            ui.label(if puente.conectado() { "Conectada" } else { "Por conectar" });
                            if ui.button(format!("Cerrar conexión {}", i + 2)).clicked() {
                                puente.cerrar();
                                *extra = None;
                            }
                        } else if ui.button(format!("Abrir conexión {}", i + 2)).clicked() {
                            match Puente::abrir() {
                                Ok(puente) => { *extra = Some(Arc::new(puente)); self.error = None; }
                                Err(e) => self.error = Some(e),
                            }
                        }
                    });
                }
                if let Some(e) = &self.error { ui.colored_label(p.red, e); }
            });
        if self.usar_tres && actual < 2 {
            ui.colored_label(p.amber, "Conecta las pestañas 2 y 3 para repartir la descarga. Mientras tanto se usa la principal.");
        }
    }

    pub fn mostrar(
        &mut self,
        ui: &mut Ui,
        p: &Pal,
        principal: Option<Arc<Puente>>,
        plan: Option<&PlanLote>,
        ocupado: bool,
        destino: &str,
        pausa: Duration,
    ) {
        egui::CollapsingHeader::new("Piloto · tres pestañas de Seguimiento").show(ui, |ui| {
            ui.label("Abre Seguimiento en las tres pestañas y conéctalas desde «1. Conexión y Alcance».");
            ui.label("Usa el mismo alcance para ambas mediciones. El piloto usa la pausa indicada, JSON para consultas vacías y los filtros del lote.");
            ui.label(RichText::new("Si las pestañas comparten sesión de SITFA, compara las filas antes de usar el resultado.").color(p.muted));
            let activa = self.activo();
            let principal = principal.filter(|p| p.conectado());
            if principal.is_none() {
                ui.label("Conecta la primera pestaña en «1. Conexión y Alcance».");
            } else {
                ui.label("Pestaña 1 · conexión de Alcance");
            }
            let alcanzable = plan.filter(|p| !p.consultas.is_empty());
            let extras: Vec<_> = self.adicionales.iter().flatten().filter(|p| p.conectado()).cloned().collect();
            let listo = !ocupado && !activa && principal.is_some() && alcanzable.is_some() && !destino.trim().is_empty();
            let listo_tres = listo && extras.len() == 2
                && alcanzable.is_some_and(|p| p.consultas.len() >= piloto::PESTANAS);
            ui.horizontal_wrapped(|ui| {
                let medir = ui.add_enabled(listo, egui::Button::new("Medir con una pestaña")).clicked();
                let probar = ui.add_enabled(listo_tres, egui::Button::new("Ejecutar con tres pestañas")).clicked();
                if let (Some(primera), Some(plan)) = (principal, alcanzable) {
                    if medir || probar {
                        let mut puentes = vec![primera];
                        if probar { puentes.extend(extras); }
                        self.iniciar(ui, puentes, plan.clone(), PathBuf::from(destino.trim()), pausa);
                    }
                }
                if let Some(cancelar) = &self.cancelar {
                    let pedido = cancelar.load(Ordering::SeqCst);
                    if ui.add_enabled(!pedido, egui::Button::new("Cancelar piloto")).clicked() {
                        cancelar.store(true, Ordering::SeqCst);
                    }
                }
                if activa { ui.spinner(); }
            });
            if alcanzable.is_some_and(|p| p.consultas.len() < piloto::PESTANAS) {
                ui.label("La ejecución con tres pestañas requiere al menos tres consultas planificadas.");
            }
            for (i, texto) in self.avance.iter().enumerate().filter(|(_, t)| !t.is_empty()) {
                ui.label(format!("Pestaña {} · {texto}", i + 1));
            }
            if let Some(error) = &self.error { ui.colored_label(p.red, error); }
            if let Some(ruta) = self.carpeta.clone().filter(|_| !activa) {
                if ui.button("Abrir carpeta del piloto").clicked() { crate::ui::abrir_en_sistema(&ruta); }
                if let Some(m) = &self.medidas {
                    ui.label(format!("{:.1} s · {} consultas · {} filas · {} XLS", m.tiempo_ms as f64 / 1000.0, m.consultas, m.registros, m.xls));
                    if m.pestanas == piloto::PESTANAS {
                        if let Some(una) = self.referencia.clone() {
                            if ui.button("Comparar con la última medición de una pestaña").clicked() { self.comparar(ui, una, ruta.clone()); }
                        }
                        if ui.button("Elegir otra medición de una pestaña…").clicked() {
                            if let Some(una) = rfd::FileDialog::new().set_title("Medición de una pestaña").pick_folder() {
                                self.comparar(ui, una, ruta);
                            }
                        }
                    }
                }
            }
            if let Some(c) = &self.comparacion {
                let paridad = c.columnas_coinciden && c.filas_nuevas == 0 && c.filas_ausentes == 0;
                let color = if paridad { p.green } else { p.amber };
                ui.colored_label(color, format!(
                    "Una: {:.1} s, {} filas · Tres: {:.1} s, {} filas · {} nuevas, {} ausentes · Columnas {}",
                    c.una.tiempo_ms as f64 / 1000.0, c.una.registros, c.tres.tiempo_ms as f64 / 1000.0, c.tres.registros,
                    c.filas_nuevas, c.filas_ausentes, if c.columnas_coinciden { "iguales" } else { "distintas" }
                ));
            }
        });
    }
}
