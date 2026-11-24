//! Generadores de Kethari y Ozh: reciben el árbol de la pregunta y escriben la
//! traducción usando solo lo que hay en la base de datos.

pub mod kethari;
pub(crate) mod lookup;
pub mod numbers;
pub mod ozh;

use numbers::NumberError;
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
