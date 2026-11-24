//! Generador de Ozh.
//!
//! Orden verbo, sujeto, objeto. Justo después del verbo va siempre una partícula
//! evidencial (`zi`, `mar` o `tho`). El pasado y el futuro se marcan con una
//! palabra al inicio, la negación con `ne` antes del verbo, la pregunta con un
//! prefijo en el verbo y el interrogativo al final. No hay plural.

use super::numbers::number_words;
use super::{GenError, Language, lookup};
use crate::ir::{Argument, Ir, NounPhrase, Quantity, SemanticSentence, Tense};
use rusqlite::Connection;

const LANG: Language = Language::Ozh;

/// Palabras y afijos de Ozh, leídos una vez por traducción.
struct Rules {
    past: String,
    future: String,
    negation: String,
    question: String,
    /// Partículas que el manual admite después del verbo.
    evidentials: Vec<String>,
}

impl Rules {
    /// Carga las reglas de la base; si falta alguna, falla en vez de suponerla.
    fn load(conn: &Connection) -> Result<Rules, GenError> {
        let order = rule(conn, "WORD_ORDER")?;
        if order != "VSO" {
            return Err(GenError::UnsupportedRule { language: LANG, rule: "WORD_ORDER", form: order });
        }
        let evidentials = lookup::rule_forms(conn, LANG, "EVIDENTIAL")?;
        if evidentials.is_empty() {
            return Err(GenError::MissingRule { language: LANG, rule: "EVIDENTIAL" });
        }
        Ok(Rules {
            past: rule(conn, "PAST")?,
            future: rule(conn, "FUTURE")?,
            negation: rule(conn, "NEGATION")?,
            question: rule(conn, "QUESTION")?,
            evidentials,
        })
    }
}

fn rule(conn: &Connection, name: &'static str) -> Result<String, GenError> {
    lookup::rule_form(conn, LANG, name)?.ok_or(GenError::MissingRule { language: LANG, rule: name })
}

/// Palabra vigente de un concepto o el motivo por el que no se puede traducir.
fn word(conn: &Connection, key: &str) -> Result<String, GenError> {
    lookup::current_word(conn, LANG, key)?.ok_or_else(|| GenError::MissingWord { language: LANG, key: key.to_string() })
}

/// Traduce el árbol a Ozh con la partícula evidencial por defecto del equipo,
/// la que está guardada en la base como `EVIDENTIAL_DEFAULT`.
pub fn generate(conn: &Connection, ir: &Ir) -> Result<String, GenError> {
    let default = rule(conn, "EVIDENTIAL_DEFAULT")?;
    generate_with(conn, ir, &default)
}

/// Igual que `generate`, pero con la partícula elegida por quien llama. Debe ser
/// una de las tres del manual: cualquier otra se rechaza.
pub fn generate_with(conn: &Connection, ir: &Ir, evidential: &str) -> Result<String, GenError> {
    let rules = Rules::load(conn)?;
    if !rules.evidentials.iter().any(|e| e == evidential) {
        return Err(GenError::UnsupportedRule { language: LANG, rule: "EVIDENTIAL", form: evidential.to_string() });
    }
    let (sentence, question) = match ir {
        Ir::Sentence(s) => (s, None),
        Ir::Question(q) => (&q.sentence, Some(q)),
    };

    let mut words = Vec::new();
    match sentence.tense {
        Tense::Past => words.push(rules.past.clone()),
        Tense::Future => words.push(rules.future.clone()),
        Tense::Present => {}
    }
    if sentence.negated {
        words.push(rules.negation.clone());
    }
    words.push(verb_form(conn, &rules, sentence, question.is_some())?);
    words.push(evidential.to_string());

    let subject = sentence.subject.as_ref().ok_or(GenError::MissingArgument("el sujeto"))?;
    words.extend(argument_words(conn, subject)?);
    if let Some(object) = &sentence.object {
        words.extend(argument_words(conn, object)?);
    }
    // El interrogativo cierra la oración, sin importar de dónde venga.
    if let Some(key) = question.and_then(|q| q.wh_word()) {
        words.push(word(conn, key)?);
    }
    Ok(words.join(" "))
}

/// Verbo, con el prefijo de pregunta cuando lo es.
fn verb_form(conn: &Connection, rules: &Rules, sentence: &SemanticSentence, is_question: bool) -> Result<String, GenError> {
    let verb = word(conn, &sentence.verb)?;
    Ok(if is_question { format!("{}{verb}", rules.question) } else { verb })
}

/// Palabras de un sujeto u objeto. Un interrogativo no aporta nada aquí porque
/// ya salió al final de la oración.
fn argument_words(conn: &Connection, arg: &Argument) -> Result<Vec<String>, GenError> {
    match arg {
        Argument::Pronoun(key) => Ok(vec![word(conn, key)?]),
        Argument::Noun(np) => noun_phrase_words(conn, np),
        Argument::Wh(_) => Ok(Vec::new()),
    }
}

/// Cantidad, sustantivo y poseedor, en ese orden. Ozh no marca el plural: lo
/// dice el número.
fn noun_phrase_words(conn: &Connection, np: &NounPhrase) -> Result<Vec<String>, GenError> {
    let mut out = Vec::new();
    // Si la cantidad es lo que se pregunta, "cuántos" ya salió al final.
    if let Some(Quantity::Exact(n)) = np.quantity {
        out.push(number_words(conn, LANG, n)?);
    }
    out.push(word(conn, &np.noun)?);
    if let Some(owner) = &np.possessor {
        out.push(word(conn, owner)?);
    }
    Ok(out)
}
