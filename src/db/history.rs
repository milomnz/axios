//! Registro de cada pregunta recibida y de lo que el sistema hizo con ella.

use super::now;
use crate::error::Result;
use rusqlite::{Connection, params};

/// Resultado para una lengua: la salida generada y si el validador la acepto.
#[derive(Debug, Clone, Default)]
pub struct LanguageRecord {
    pub output: Option<String>,
    pub valid: bool,
    pub error: Option<String>,
}

/// Todo lo que se guarda al terminar de procesar una pregunta.
#[derive(Debug, Clone, Default)]
pub struct QuestionRecord {
    pub normalized: Option<String>,
    pub corrected: Option<String>,
    pub kethari: LanguageRecord,
    pub ozh: LanguageRecord,
    /// Motivo cuando el proceso se detuvo antes de llegar a los generadores.
    pub error_reason: Option<String>,
}

#[derive(Debug)]
pub struct HistoryEntry {
    pub id: i64,
    pub created_at: String,
    pub raw_input: String,
    pub corrected_input: Option<String>,
    pub kethari_output: Option<String>,
    pub kethari_valid: Option<bool>,
    pub kethari_error: Option<String>,
    pub ozh_output: Option<String>,
    pub ozh_valid: Option<bool>,
    pub ozh_error: Option<String>,
    pub status: String,
    pub error_reason: Option<String>,
}

/// Guarda la pregunta tal como llego, antes de intentar traducirla.
/// Devuelve su id y la hora de registro.
pub fn register_question(conn: &Connection, raw_input: &str) -> Result<(i64, String)> {
    let at = now(conn)?;
    conn.execute(
        "INSERT INTO translations (created_at, raw_input, status) VALUES (?1, ?2, 'RECEIVED')",
        params![at, raw_input],
    )?;
    Ok((conn.last_insert_rowid(), at))
}

/// Completa el registro de una pregunta y devuelve el estado final.
pub fn finish_question(conn: &Connection, id: i64, record: &QuestionRecord) -> Result<&'static str> {
    let status = match (record.kethari.valid, record.ozh.valid) {
        (true, true) => "TRANSLATED",
        (true, false) | (false, true) => "PARTIAL",
        (false, false) => "REJECTED",
    };
    // Una salida que no paso la validacion se guarda para poder auditarla,
    // pero el estado deja claro que no se transmite.
    conn.execute(
        "UPDATE translations
            SET normalized_input = ?2, corrected_input = ?3,
                kethari_output = ?4, kethari_valid = ?5, kethari_error = ?6,
                ozh_output = ?7, ozh_valid = ?8, ozh_error = ?9,
                status = ?10, error_reason = ?11
          WHERE id = ?1",
        params![
            id,
            record.normalized,
            record.corrected,
            record.kethari.output,
            record.kethari.output.as_ref().map(|_| record.kethari.valid),
            record.kethari.error,
            record.ozh.output,
            record.ozh.output.as_ref().map(|_| record.ozh.valid),
            record.ozh.error,
            status,
            record.error_reason,
        ],
    )?;
    Ok(status)
}

/// Preguntas registradas desde `since` (incluida), en orden de llegada.
pub fn list_history(conn: &Connection, since: Option<&str>) -> Result<Vec<HistoryEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, created_at, raw_input, corrected_input,
                kethari_output, kethari_valid, kethari_error,
                ozh_output, ozh_valid, ozh_error, status, error_reason
           FROM translations
          WHERE ?1 IS NULL OR created_at >= ?1
          ORDER BY created_at, id",
    )?;
    let rows = stmt.query_map([since], |r| {
        Ok(HistoryEntry {
            id: r.get(0)?,
            created_at: r.get(1)?,
            raw_input: r.get(2)?,
            corrected_input: r.get(3)?,
            kethari_output: r.get(4)?,
            kethari_valid: r.get(5)?,
            kethari_error: r.get(6)?,
            ozh_output: r.get(7)?,
            ozh_valid: r.get(8)?,
            ozh_error: r.get(9)?,
            status: r.get(10)?,
            error_reason: r.get(11)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}
