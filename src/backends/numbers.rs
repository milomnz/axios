//! Conversión de números a palabras en Kethari y Ozh.
//!
//! Las dos lenguas encadenan las cifras con un separador. La base (6 en
//! Kethari, 10 en Ozh), el separador y el nombre de cada cifra se leen de la
//! base de datos, así una corrección del manual no obliga a tocar el código.

use super::{Language, lookup};
use rusqlite::{Connection, OptionalExtension, params};
use std::fmt;

/// Mayor cantidad que el enunciado pide traducir.
pub const MAX_NUMBER: u32 = 9_999;

#[derive(Debug)]
pub enum NumberError {
    /// El número pasa del máximo permitido.
    OutOfRange(u32),
    /// La base de datos no tiene la regla (`NUMBER_BASE` o `NUMBER_SEPARATOR`) de la lengua.
    MissingRule { language: Language, rule: &'static str },
    /// La lengua no tiene palabra para una cifra que el número necesita.
    MissingDigit { language: Language, digit: u32 },
    Db(rusqlite::Error),
}

impl fmt::Display for NumberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NumberError::OutOfRange(n) => write!(f, "{n} pasa del máximo de {MAX_NUMBER}"),
            NumberError::MissingRule { language, rule } => {
                write!(f, "no hay regla {rule} para {}", language.db_name())
            }
            NumberError::MissingDigit { language, digit } => {
                write!(f, "{} no tiene palabra para la cifra {digit}", language.db_name())
            }
            NumberError::Db(e) => write!(f, "error de base de datos: {e}"),
        }
    }
}

impl std::error::Error for NumberError {}

impl From<rusqlite::Error> for NumberError {
    fn from(e: rusqlite::Error) -> Self {
        NumberError::Db(e)
    }
}

/// Escribe `value` en la lengua indicada: sus cifras en la base de esa lengua,
/// cada una con su palabra y unidas por el separador ("quince" en Ozh es `ois-ulo`).
pub fn number_words(conn: &Connection, language: Language, value: u32) -> Result<String, NumberError> {
    if value > MAX_NUMBER {
        return Err(NumberError::OutOfRange(value));
    }
    let base: u32 = lookup::rule_form(conn, language, "NUMBER_BASE")?
        .and_then(|form| form.parse().ok())
        .ok_or(NumberError::MissingRule { language, rule: "NUMBER_BASE" })?;
    let separator = lookup::rule_form(conn, language, "NUMBER_SEPARATOR")?
        .ok_or(NumberError::MissingRule { language, rule: "NUMBER_SEPARATOR" })?;

    let words: Result<Vec<String>, NumberError> =
        digits(value, base).into_iter().map(|d| digit_word(conn, language, d)).collect();
    Ok(words?.join(&separator))
}

/// Cifras de `value` en la base dada, de la más significativa a la menos.
/// El cero se escribe como una sola cifra.
fn digits(mut value: u32, base: u32) -> Vec<u32> {
    let mut out = Vec::new();
    loop {
        out.push(value % base);
        value /= base;
        if value == 0 {
            break;
        }
    }
    out.reverse();
    out
}

/// Palabra vigente de una cifra en la lengua, o error si el manual no la trae.
fn digit_word(conn: &Connection, language: Language, digit: u32) -> Result<String, NumberError> {
    conn.query_row(
        "SELECT l.word FROM lexicon l JOIN concepts c ON c.id = l.concept_id
          WHERE c.category = 'NUMBER' AND c.value = ?1 AND l.language = ?2 AND l.valid_to IS NULL",
        params![digit, language.db_name()],
        |r| r.get(0),
    )
    .optional()?
    .ok_or(NumberError::MissingDigit { language, digit })
}

#[cfg(test)]
mod tests {
    use super::digits;

    #[test]
    fn cifras_en_base_6() {
        assert_eq!(digits(15, 6), vec![2, 3]);
        assert_eq!(digits(6, 6), vec![1, 0]);
        assert_eq!(digits(26, 6), vec![4, 2]);
        assert_eq!(digits(9_999, 6), vec![1, 1, 4, 1, 4, 3]);
    }

    #[test]
    fn cifras_en_base_10() {
        assert_eq!(digits(15, 10), vec![1, 5]);
        assert_eq!(digits(2_015, 10), vec![2, 0, 1, 5]);
    }

    #[test]
    fn el_cero_es_una_sola_cifra() {
        assert_eq!(digits(0, 6), vec![0]);
        assert_eq!(digits(0, 10), vec![0]);
    }
}
