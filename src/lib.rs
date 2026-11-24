pub mod cli;
pub mod backends;
pub mod db;
pub mod error;
pub mod ir;

use db::{LanguageRecord, QuestionRecord};
use rusqlite::Connection;

/// Salida de una fase, para el modo traza.
#[derive(Debug, Clone)]
pub struct Stage {
    pub name: String,
    pub output: String,
}

/// Resultado del pipeline. Lo que la CLI necesita para mostrar y registrar.
#[derive(Debug, Clone, Default)]
pub struct PipelineOutput {
    pub record: QuestionRecord,
    pub trace: Vec<Stage>,
}

/// Traduce una pregunta en español a Kethari y Ozh.
///
/// Aqui se encadenan las fases: normalizador, lexer, corrector, morfologia,
/// parser, IR, generadores y validadores. Mientras una fase no este integrada,
/// la pregunta se rechaza con el motivo y queda registrada igual.
pub fn translate(conn: &Connection, input: &str) -> PipelineOutput {
    let _ = (conn, input);
    PipelineOutput {
        record: QuestionRecord {
            kethari: LanguageRecord::default(),
            ozh: LanguageRecord::default(),
            error_reason: Some("el analizador del español todavia no esta integrado".into()),
            ..Default::default()
        },
        trace: Vec::new(),
    }
}
