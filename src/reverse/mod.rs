//! Traducción de regreso: de Kethari u Ozh al español (bonus de la rúbrica).
//!
//! Se apoya en los validadores, que ya entienden cada lengua y entregan el
//! árbol de lo que leen. Aquí el árbol se escribe en español con el léxico y
//! las desinencias de la base de datos. El español de la base no lleva tildes,
//! así que la salida tampoco; tampoco hay artículos ni concordancia de generó,
//! porque ninguna de las dos lenguas los tiene.

use crate::backends::Language;
use crate::ir::{Argument, Ir, NounPhrase, Quantity, SemanticSentence, Tense};
use crate::validation::{self, ValidationError};
use rusqlite::{Connection, OptionalExtension, params};
use std::fmt;

#[derive(Debug)]
pub enum ReverseError {
    /// La frase no es válida en la lengua de origen.
    Invalid(ValidationError),
    /// La base no tiene español para un concepto.
    MissingSpanish(String),
    /// Falta la desinencia o la forma para conjugar un verbo.
    MissingForm { verb: String },
    Db(rusqlite::Error),
}

impl fmt::Display for ReverseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReverseError::Invalid(e) => write!(f, "la frase no es válida: {e}"),
            ReverseError::MissingSpanish(key) => write!(f, "la base no tiene español para {key}"),
            ReverseError::MissingForm { verb } => write!(f, "no se pudo conjugar «{verb}»"),
            ReverseError::Db(e) => write!(f, "error de base de datos: {e}"),
        }
    }
}

impl std::error::Error for ReverseError {}

impl From<rusqlite::Error> for ReverseError {
    fn from(e: rusqlite::Error) -> Self {
        ReverseError::Db(e)
    }
}

impl From<ValidationError> for ReverseError {
    fn from(e: ValidationError) -> Self {
        match e {
            ValidationError::Db(db) => ReverseError::Db(db),
            other => ReverseError::Invalid(other),
        }
    }
}

/// Lee `text` en la lengua dada y lo escribe en español. Devuelve una frase por
/// cada lectura válida, la preferida primero: una frase ambigua tiene varias.
pub fn to_spanish(conn: &Connection, language: Language, text: &str) -> Result<Vec<String>, ReverseError> {
    let reading = match language {
        Language::Kethari => validation::kethari::validate(conn, text)?,
        Language::Ozh => validation::ozh::validate(conn, text)?,
    };
    std::iter::once(&reading.ir).chain(&reading.alternatives).map(|ir| render(conn, ir)).collect()
}

/// Escribe un árbol en español.
fn render(conn: &Connection, ir: &Ir) -> Result<String, ReverseError> {
    let (sentence, is_question) = match ir {
        Ir::Sentence(s) => (s, false),
        Ir::Question(q) => (&q.sentence, true),
    };
    let mut words = Vec::new();
    // El interrogativo, incluidos "cuándo" y "dónde", abre la pregunta.
    if let Some(key) = ir_wh(ir) {
        words.push(spanish(conn, key)?);
    }
    let subject_words = sentence.subject.as_ref().map(|a| argument(conn, a)).transpose()?.unwrap_or_default();
    let mut object_words = sentence.object.as_ref().map(|a| argument(conn, a)).transpose()?.unwrap_or_default();
    // Un pronombre de persona como objeto lleva "a": "vimos a ustedes".
    if matches!(sentence.object, Some(Argument::Pronoun(_))) {
        object_words.insert(0, "a".to_string());
    }
    let verb = conjugate(conn, sentence)?;
    let verb_words: Vec<String> = if sentence.negated { vec![spanish_negation(conn)?, verb] } else { vec![verb] };

    if is_question {
        // Pregunta: interrogativo, verbo, sujeto y objeto.
        words.extend(verb_words);
        words.extend(subject_words);
        words.extend(object_words);
    } else {
        words.extend(subject_words);
        words.extend(verb_words);
        words.extend(object_words);
    }
    let text = words.join(" ");
    Ok(if is_question { format!("¿{}?", capitalize(&text)) } else { format!("{}.", capitalize(&text)) })
}

/// Palabra interrogativa de la frase, si la hay.
fn ir_wh(ir: &Ir) -> Option<&str> {
    match ir {
        Ir::Question(q) => q.wh_word(),
        Ir::Sentence(_) => None,
    }
}

/// Palabras de un sujeto u objeto. Un interrogativo no aporta nada: ya abrió la frase.
fn argument(conn: &Connection, arg: &Argument) -> Result<Vec<String>, ReverseError> {
    match arg {
        Argument::Pronoun(key) => Ok(vec![spanish(conn, key)?]),
        Argument::Noun(np) => noun_phrase(conn, np),
        Argument::Wh(_) => Ok(Vec::new()),
    }
}

/// Cantidad, sustantivo y poseedor ("2 mensajes", "planeta de nosotros").
fn noun_phrase(conn: &Connection, np: &NounPhrase) -> Result<Vec<String>, ReverseError> {
    let mut out = Vec::new();
    if let Some(Quantity::Exact(n)) = np.quantity {
        out.push(n.to_string());
    }
    let noun = spanish(conn, &np.noun)?;
    out.push(if np.is_plural() { plural(&noun) } else { noun });
    if let Some(owner) = &np.possessor {
        out.push("de".to_string());
        out.push(spanish(conn, owner)?);
    }
    Ok(out)
}

/// Plural regular del español: vocal + s, z se vuelve ces, consonante + es.
fn plural(noun: &str) -> String {
    match noun.chars().last() {
        Some('a' | 'e' | 'i' | 'o' | 'u') => format!("{noun}s"),
        Some('z') => format!("{}ces", &noun[..noun.len() - 1]),
        _ => format!("{noun}es"),
    }
}

/// Primera palabra vigente en español de un concepto.
fn spanish(conn: &Connection, key: &str) -> Result<String, ReverseError> {
    conn.query_row(
        "SELECT l.word FROM lexicon l JOIN concepts c ON c.id = l.concept_id
          WHERE c.semantic_key = ?1 AND l.language = 'ES' AND l.valid_to IS NULL ORDER BY l.id LIMIT 1",
        [key],
        |r| r.get(0),
    )
    .optional()?
    .ok_or_else(|| ReverseError::MissingSpanish(key.to_string()))
}

fn spanish_negation(conn: &Connection) -> Result<String, ReverseError> {
    conn.query_row("SELECT word FROM es_function_words WHERE kind = 'NEGATION' LIMIT 1", [], |r| r.get(0))
        .optional()?
        .ok_or_else(|| ReverseError::MissingSpanish("negacion".to_string()))
}

/// Persona y número con que se conjuga el verbo según el sujeto. "Ustedes"
/// conjuga como tercera persona del plural.
fn person_number(subject: Option<&Argument>) -> (i64, &'static str) {
    match subject {
        Some(Argument::Pronoun(key)) => match key.as_str() {
            "YO" => (1, "SG"),
            "TU" => (2, "SG"),
            "NOSOTROS" => (1, "PL"),
            "USTEDES" | "ELLOS" => (3, "PL"),
            _ => (3, "SG"),
        },
        Some(Argument::Noun(np)) if np.is_plural() => (3, "PL"),
        _ => (3, "SG"),
    }
}

/// Verbo conjugado: primero las formas irregulares de la base y si no hay, las
/// desinencias regulares de su conjugación.
fn conjugate(conn: &Connection, sentence: &SemanticSentence) -> Result<String, ReverseError> {
    let infinitive = spanish(conn, &sentence.verb)?;
    let tense = match sentence.tense {
        Tense::Past => "PAST",
        Tense::Present => "PRESENT",
        Tense::Future => "FUTURE",
    };
    let (person, number) = person_number(sentence.subject.as_ref());
    let irregular = conn
        .query_row(
            "SELECT form FROM es_irregular_forms WHERE infinitive = ?1 AND tense = ?2 AND person = ?3 AND number = ?4",
            params![infinitive, tense, person, number],
            |r| r.get::<_, String>(0),
        )
        .optional()?;
    if let Some(form) = irregular {
        return Ok(form);
    }
    let conjugation = &infinitive[infinitive.len().saturating_sub(2)..];
    let ending: Option<(String, String)> = conn
        .query_row(
            "SELECT ending, attaches_to FROM es_verb_endings
              WHERE conjugation IN (?1, '*') AND tense = ?2 AND person = ?3 AND number = ?4",
            params![conjugation, tense, person, number],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    match ending {
        Some((ending, attaches_to)) if attaches_to == "INFINITIVE" => Ok(format!("{infinitive}{ending}")),
        Some((ending, _)) => Ok(format!("{}{ending}", &infinitive[..infinitive.len() - 2])),
        None => Err(ReverseError::MissingForm { verb: infinitive }),
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}
