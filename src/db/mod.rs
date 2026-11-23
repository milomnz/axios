//! Persistencia en SQLite: esquema, carga de archivos, lexico versionado e historial.

mod history;
mod lexicon;
mod loader;

pub use history::{HistoryEntry, LanguageRecord, QuestionRecord, finish_question, list_history, register_question};
pub use lexicon::{ConceptView, RowVersion, SignalMatch, WordReport, spanish_word_exists, spanish_words, word_report};
pub use loader::{LoadReport, load_file};

use crate::error::Result;
use rusqlite::Connection;
use rusqlite::functions::FunctionFlags;
use std::path::Path;

/// Abre (o crea) la base de datos y garantiza que el esquema exista.
pub fn open(path: &Path) -> Result<Connection> {
    init(Connection::open(path)?)
}

pub fn open_in_memory() -> Result<Connection> {
    init(Connection::open_in_memory()?)
}

fn init(conn: Connection) -> Result<Connection> {
    conn.execute_batch(include_str!("schema.sql"))?;
    // fold() en SQL, para buscar palabras sin importar las tildes.
    conn.create_scalar_function("fold", 1, FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC, |ctx| {
        Ok(fold(&ctx.get::<String>(0)?))
    })?;
    Ok(conn)
}

/// Hora actual segun SQLite, con milisegundos, en hora local.
pub(crate) fn now(conn: &Connection) -> Result<String> {
    Ok(conn.query_row("SELECT strftime('%Y-%m-%d %H:%M:%f', 'now', 'localtime')", [], |r| r.get(0))?)
}

/// Completa una fecha escrita por el usuario hasta el formato de la base.
/// Acepta `AAAA-MM-DD`, `AAAA-MM-DD HH:MM`, `AAAA-MM-DD HH:MM:SS` y con milisegundos;
/// el separador puede ser espacio o `T`. Lo que falta se rellena con el inicio del
/// periodo (`upper = false`) o con su final (`upper = true`).
pub fn complete_date(input: &str, upper: bool) -> Option<String> {
    const LOWER: &str = "0000-01-01 00:00:00.000";
    const UPPER: &str = "0000-12-31 23:59:59.999";
    const SHAPE: &str = "dddd-dd-dd dd:dd:dd.ddd";
    let s = input.trim().replacen('T', " ", 1);
    if ![10, 16, 19, 23].contains(&s.len()) {
        return None;
    }
    let shape_ok = s.chars().zip(SHAPE.chars()).all(|(c, p)| if p == 'd' { c.is_ascii_digit() } else { c == p });
    if !shape_ok {
        return None;
    }
    let full = format!("{s}{}", &(if upper { UPPER } else { LOWER })[s.len()..]);
    let n = |a: usize, b: usize| full[a..b].parse::<u32>().unwrap();
    let valid = (1..=12).contains(&n(5, 7)) && (1..=31).contains(&n(8, 10)) && n(11, 13) < 24 && n(14, 16) < 60 && n(17, 19) < 60;
    valid.then_some(full)
}

/// Codigo de lengua que se guarda en la base.
pub(crate) fn language_code(name: &str) -> Option<&'static str> {
    match name.trim().to_lowercase().as_str() {
        "es" | "español" | "espanol" => Some("ES"),
        "kethari" => Some("KETHARI"),
        "ozh" => Some("OZH"),
        _ => None,
    }
}

/// Minusculas y vocales sin tilde. Solo se usa para comparar significados
/// y busquedas; las palabras se guardan tal como vienen.
pub(crate) fn fold(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        // Codigos Unicode de las vocales con tilde (a, e, i, o, u) y de la u con dieresis.
        .map(|c| match c {
            '\u{e1}' => 'a',
            '\u{e9}' => 'e',
            '\u{ed}' => 'i',
            '\u{f3}' => 'o',
            '\u{fa}' | '\u{fc}' => 'u',
            other => other,
        })
        .collect()
}

/// Clave de concepto a partir de un significado: "territorio reclamado" -> TERRITORIO_RECLAMADO.
pub(crate) fn concept_key(meaning: &str) -> String {
    let mut key = String::new();
    for c in fold(meaning).chars() {
        match c {
            'ñ' => key.push_str("NI"),
            c if c.is_ascii_alphanumeric() => key.push(c.to_ascii_uppercase()),
            _ if !key.ends_with('_') && !key.is_empty() => key.push('_'),
            _ => {}
        }
    }
    key.trim_end_matches('_').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completa_fechas() {
        assert_eq!(complete_date("2026-11-20", false).unwrap(), "2026-11-20 00:00:00.000");
        assert_eq!(complete_date("2026-11-20", true).unwrap(), "2026-11-20 23:59:59.999");
        assert_eq!(complete_date("2026-11-20T10:30", true).unwrap(), "2026-11-20 10:30:59.999");
        assert!(complete_date("20-11-2026", false).is_none());
        assert!(complete_date("2026-13-01", false).is_none());
        assert!(complete_date("2026-11-20 25:00", false).is_none());
    }

    #[test]
    fn claves_de_concepto() {
        assert_eq!(concept_key("territorio reclamado"), "TERRITORIO_RECLAMADO");
        assert_eq!(concept_key("año"), "ANIO");
        assert_eq!(concept_key("¿Que?"), "QUE");
    }
}
