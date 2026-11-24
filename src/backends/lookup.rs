//! Consultas a la base que comparten los generadores: la palabra vigente de un
//! concepto en una lengua y la forma de una regla gramatical.

use super::Language;
use rusqlite::{Connection, OptionalExtension, params};

/// Palabra vigente de un concepto en la lengua. Si el manual trae varias formas
/// (como `anul` y `anir`), se usa la primera que se cargó.
pub fn current_word(conn: &Connection, language: Language, key: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT l.word FROM lexicon l JOIN concepts c ON c.id = l.concept_id
          WHERE c.semantic_key = ?1 AND l.language = ?2 AND l.valid_to IS NULL
          ORDER BY l.id LIMIT 1",
        params![key, language.db_name()],
        |r| r.get(0),
    )
    .optional()
}

/// Forma de una regla de la lengua, por ejemplo `ok` para `PAST` en Kethari.
pub fn rule_form(conn: &Connection, language: Language, rule: &str) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT form FROM grammar_rules WHERE language = ?1 AND rule = ?2 ORDER BY id LIMIT 1",
        params![language.db_name(), rule],
        |r| r.get(0),
    )
    .optional()
}

/// Todas las formas de una regla que puede repetirse, como las tres partículas
/// evidenciales de Ozh, en el orden en que se cargaron.
pub fn rule_forms(conn: &Connection, language: Language, rule: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT form FROM grammar_rules WHERE language = ?1 AND rule = ?2 ORDER BY id")?;
    let forms = stmt.query_map(params![language.db_name(), rule], |r| r.get(0))?.collect();
    forms
}
