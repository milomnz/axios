use std::fmt;

#[derive(Debug)]
pub enum AxiosError {
    Db(rusqlite::Error),
    Io { path: String, source: std::io::Error },
    /// Una linea de un archivo que se esta cargando no se pudo incorporar.
    /// La carga completa se deshace.
    Load { file: String, line: usize, message: String },
}

pub type Result<T> = std::result::Result<T, AxiosError>;

impl fmt::Display for AxiosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AxiosError::Db(e) => write!(f, "error de base de datos: {e}"),
            AxiosError::Io { path, source } => write!(f, "no se pudo leer {path}: {source}"),
            AxiosError::Load { file, line, message } => {
                write!(f, "{file}, linea {line}: {message}. No se cargo nada del archivo")
            }
        }
    }
}

impl std::error::Error for AxiosError {}

impl From<rusqlite::Error> for AxiosError {
    fn from(e: rusqlite::Error) -> Self {
        AxiosError::Db(e)
    }
}
