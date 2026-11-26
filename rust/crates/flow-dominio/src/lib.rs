//! Tipos y utilidades de texto y fecha compartidos por todo Flow.
//! Sin E/S. La lógica replica `src/nurus/personal/motor/utilidades.py` y `src/nurus/rus/columns.py`.

pub mod celda;
pub mod evidencia;
pub mod historia;
pub mod texto;

pub use celda::{Celda, Registro};
pub use evidencia::{Escrito, Escritos, EvidenciaInforme, Historico, ProgramaHistorico};
pub use chrono::NaiveDate as Fecha;
pub use historia::{Historia, Observacion, Tramite};

use serde::{Deserialize, Serialize};

/// CC obligatorio de todo correo. No es configurable.
pub const CC_FIJO: &str = "ucc_concepcion@pjud.cl";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Etapa {
    Espera,
    Cumplimiento,
    Informes,
}

impl Etapa {
    pub fn nombre(self) -> &'static str {
        match self {
            Etapa::Espera => "ESPERA",
            Etapa::Cumplimiento => "CUMPLIMIENTO",
            Etapa::Informes => "INFORMES",
        }
    }
    pub fn desde(texto: &str) -> Option<Self> {
        match texto::normalizar(texto).as_str() {
            "espera" => Some(Etapa::Espera),
            "cumplimiento" => Some(Etapa::Cumplimiento),
            "informes" | "informe" => Some(Etapa::Informes),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Tribunal {
    Laja,
    Mulchen,
    Tome,
}

impl Tribunal {
    pub fn nombre(self) -> &'static str {
        match self {
            Tribunal::Laja => "LAJA",
            Tribunal::Mulchen => "MULCHEN",
            Tribunal::Tome => "TOME",
        }
    }
    pub fn titulo(self) -> &'static str {
        match self {
            Tribunal::Laja => "Laja",
            Tribunal::Mulchen => "Mulchén",
            Tribunal::Tome => "Tomé",
        }
    }
    /// Reconoce el tribunal inequívoco dentro del texto de la celda (igual que `rus/util.py::tribunal`).
    pub fn desde(texto: &str) -> Option<Self> {
        let bruto = texto.trim();
        if bruto.is_empty() || es_compuesto_ambiguo(bruto) {
            return None;
        }
        let norm = normalizar_tribunal(bruto);
        let palabras: Vec<&str> = norm
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();
        let mut conocido = None;
        for (tok, trib) in [("mulchen", Tribunal::Mulchen), ("laja", Tribunal::Laja), ("tome", Tribunal::Tome)] {
            if palabras.contains(&tok) {
                if conocido.is_some() {
                    return None;
                }
                conocido = Some(trib);
            }
        }
        conocido
    }

    /// Resuelve el perfil de reglas (`Laja`, `Mulchen` o `Tome`), idéntico a `motor/utilidades.py::perfil_tribunal`.
    /// Cualquier otro tribunal válido usa el perfil `Tome`; vacíos, marcadores de ausencia y rótulos ambiguos devuelven `None`.
    pub fn perfil(texto: &str) -> Option<Self> {
        let bruto = texto.trim();
        if bruto.is_empty() || es_compuesto_ambiguo(bruto) {
            return None;
        }
        let norm = normalizar_tribunal(bruto);
        if norm.bytes().filter(|b| b.is_ascii_alphabetic()).count() < 3 {
            return None;
        }
        if MARCADORES_AUSENCIA_TRIBUNAL.contains(&norm.as_str()) {
            return None;
        }
        let palabras: Vec<&str> = norm
            .split(|c: char| !c.is_ascii_alphanumeric())
            .filter(|s| !s.is_empty())
            .collect();
        let mut conocido = None;
        for (tok, trib) in [("mulchen", Tribunal::Mulchen), ("laja", Tribunal::Laja), ("tome", Tribunal::Tome)] {
            if palabras.contains(&tok) {
                if conocido.is_some() {
                    return None;
                }
                conocido = Some(trib);
            }
        }
        Some(conocido.unwrap_or(Tribunal::Tome))
    }
}

const MARCADORES_AUSENCIA_TRIBUNAL: &[&str] = &[
    "nan", "nat", "none", "null", "na", "n a", "s i", "s d", "s n", "sd", "sn",
    "sin dato", "sin datos", "sin informacion", "sin tribunal", "sin asignar",
    "sin especificar", "sin registro", "no informado", "no aplica", "no indica",
    "no especificado", "no identificado", "no disponible", "no registra",
    "ninguno", "ninguna", "desconocido", "desconocida", "indeterminado",
    "pendiente", "por definir", "por confirmar", "otro", "otros", "varios",
    "multiple", "todos", "total",
];

fn es_compuesto_ambiguo(bruto: &str) -> bool {
    if !bruto.contains(['/', '|', ';']) {
        return false;
    }
    bruto
        .split(['/', '|', ';'])
        .filter(|p| texto::normalizar(p).bytes().any(|b| b.is_ascii_alphabetic()))
        .count()
        >= 2
}

fn normalizar_tribunal(bruto: &str) -> String {
    let norm = texto::normalizar(bruto);
    let limpio: String = norm
        .chars()
        .map(|c| {
            if matches!(c, '-' | '(' | ')' | '.' | '_' | ',' | ':' | '/' | '\\' | '|' | ';') {
                ' '
            } else {
                c
            }
        })
        .collect();
    limpio.split_whitespace().collect::<Vec<_>>().join(" ")
}
