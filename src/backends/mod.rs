//! Generadores de Kethari y Ozh: reciben el árbol de la pregunta y escriben la
//! traducción usando solo lo que hay en la base de datos.

pub(crate) mod lookup;
pub mod numbers;

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
