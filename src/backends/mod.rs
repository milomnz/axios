//! Generadores de Kethari y Ozh: reciben el árbol de la pregunta y escriben la
//! traducción usando solo lo que hay en la base de datos.

pub mod kethari;
pub(crate) mod lookup;
pub mod numbers;
pub mod ozh;

use crate::Stage;
use crate::db::LanguageRecord;
use crate::ir::Ir;
use crate::validation::{self, ValidationError};
use numbers::NumberError;
use rusqlite::Connection;
use std::fmt;

/// Lenguas alienígenas a las que se traduce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Kethari,
    Ozh,
}

impl Language {
    /// Nombre de la lengua tal como se guarda en las tablas `lexicon` y `grammar_rules`.
    pub fn db_name(self) -> &'static str {
        match self {
            Language::Kethari => "KETHARI",
            Language::Ozh => "OZH",
        }
    }
}

/// Motivos por los que un generador no puede producir una traducción.
/// Cada uno se puede mostrar tal cual al funcionario que hizo la pregunta.
#[derive(Debug)]
pub enum GenError {
    /// El manual no tiene (o ya no respalda) una palabra para este concepto.
    MissingWord { language: Language, key: String },
    /// Falta una regla gramatical en la base.
    MissingRule { language: Language, rule: &'static str },
    /// La oración llegó sin un elemento que la lengua exige.
    MissingArgument(&'static str),
    /// La base trae una regla que este generador no sabe aplicar.
    UnsupportedRule { language: Language, rule: &'static str, form: String },
    Number(NumberError),
    Db(rusqlite::Error),
}

impl fmt::Display for GenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GenError::MissingWord { language, key } => {
                write!(f, "{} no tiene palabra para {key} en el manual", language.db_name())
            }
            GenError::MissingRule { language, rule } => {
                write!(f, "no hay regla {rule} para {}", language.db_name())
            }
            GenError::MissingArgument(what) => write!(f, "falta {what} en la oración"),
            GenError::UnsupportedRule { language, rule, form } => {
                write!(f, "la regla {rule}={form} de {} no está soportada", language.db_name())
            }
            GenError::Number(e) => write!(f, "{e}"),
            GenError::Db(e) => write!(f, "error de base de datos: {e}"),
        }
    }
}

impl std::error::Error for GenError {}

impl From<rusqlite::Error> for GenError {
    fn from(e: rusqlite::Error) -> Self {
        GenError::Db(e)
    }
}

impl From<NumberError> for GenError {
    fn from(e: NumberError) -> Self {
        GenError::Number(e)
    }
}

/// Lo que sale de traducir un árbol: el resultado de cada lengua y las fases
/// recorridas, listas para el modo traza.
#[derive(Debug, Clone, Default)]
pub struct IrTranslation {
    pub kethari: LanguageRecord,
    pub ozh: LanguageRecord,
    pub trace: Vec<Stage>,
}

/// Traduce el árbol a las dos lenguas y pasa cada salida por su validador.
/// Una lengua que falla no impide la otra: cada una lleva su propio resultado,
/// con el motivo cuando no hay traducción o el validador la rechaza.
pub fn translate_ir(conn: &Connection, ir: &Ir) -> IrTranslation {
    let mut trace = Vec::new();
    let kethari = finish_language(
        "Kethari",
        kethari::generate(conn, ir),
        |text| validation::kethari::validate(conn, text).map(|_| ()),
        &mut trace,
    );
    let ozh = finish_language(
        "Ozh",
        ozh::generate(conn, ir),
        |text| validation::ozh::validate(conn, text).map(|_| ()),
        &mut trace,
    );
    IrTranslation { kethari, ozh, trace }
}

/// Une generación y validación de una lengua en un solo resultado y anota las
/// dos fases en la traza.
fn finish_language(
    name: &str,
    generated: Result<String, GenError>,
    validate: impl Fn(&str) -> Result<(), ValidationError>,
    trace: &mut Vec<Stage>,
) -> LanguageRecord {
    match generated {
        Err(e) => {
            trace.push(Stage { name: format!("Generador {name}"), output: format!("sin traducción: {e}") });
            LanguageRecord { output: None, valid: false, error: Some(e.to_string()) }
        }
        Ok(text) => {
            trace.push(Stage { name: format!("Generador {name}"), output: text.clone() });
            match validate(&text) {
                Ok(()) => {
                    trace.push(Stage { name: format!("Validador {name}"), output: "valida".to_string() });
                    LanguageRecord { output: Some(text), valid: true, error: None }
                }
                Err(e) => {
                    trace.push(Stage { name: format!("Validador {name}"), output: format!("rechazada: {e}") });
                    LanguageRecord { output: Some(text), valid: false, error: Some(e.to_string()) }
                }
            }
        }
    }
}
