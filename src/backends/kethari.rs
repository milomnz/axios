//! Generador de Kethari.
//!
//! Orden sujeto, objeto, verbo. El tiempo y la negación van en el verbo, el
//! plural en el sustantivo, y el interrogativo se mueve al inicio de la oración
//! con la partícula `ka` al final. Las partículas y afijos se leen de la base.

use super::numbers::number_words;
use super::{GenError, Language, lookup};
use crate::ir::{Argument, Ir, NounPhrase, Quantity, SemanticSentence, Tense};
use rusqlite::Connection;

const LANG: Language = Language::Kethari;

/// Afijos y partículas de Kethari, leídos una vez por traducción.
struct Rules {
    past: String,
    future: String,
    negation: String,
    plural: String,
    question: String,
}

impl Rules {
    /// Carga las reglas de la base; si falta alguna, falla en vez de suponerla.
    fn load(conn: &Connection) -> Result<Rules, GenError> {
        let order = rule(conn, "WORD_ORDER")?;
        if order != "SOV" {
            return Err(GenError::UnsupportedRule { language: LANG, rule: "WORD_ORDER", form: order });
        }
        Ok(Rules {
            past: rule(conn, "PAST")?,
            future: rule(conn, "FUTURE")?,
            negation: rule(conn, "NEGATION")?,
            plural: rule(conn, "PLURAL")?,
            question: rule(conn, "QUESTION")?,
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

/// Traduce el árbol a Kethari. Devuelve la oración lista para transmitir o el
/// motivo por el que no existe traducción.
pub fn generate(conn: &Connection, ir: &Ir) -> Result<String, GenError> {
    let rules = Rules::load(conn)?;
    let (sentence, question) = match ir {
        Ir::Sentence(s) => (s, None),
        Ir::Question(q) => (&q.sentence, Some(q)),
    };

    let mut words = Vec::new();
    // El interrogativo abre la oración, sin importar de dónde venga.
    if let Some(key) = question.and_then(|q| q.wh_word()) {
        words.push(word(conn, key)?);
    }
    let subject = sentence.subject.as_ref().ok_or(GenError::MissingArgument("el sujeto"))?;
    words.extend(argument_words(conn, &rules, subject)?);
    if let Some(object) = &sentence.object {
        words.extend(argument_words(conn, &rules, object)?);
    }
    words.push(verb_form(conn, &rules, sentence)?);
    if question.is_some() {
        words.push(rules.question.clone());
    }
    Ok(words.join(" "))
}

/// Palabras de un sujeto u objeto. Un interrogativo no aporta nada aquí porque
/// ya salió al inicio de la oración.
fn argument_words(conn: &Connection, rules: &Rules, arg: &Argument) -> Result<Vec<String>, GenError> {
    match arg {
        Argument::Pronoun(key) => Ok(vec![word(conn, key)?]),
        Argument::Noun(np) => noun_phrase_words(conn, rules, np),
        Argument::Wh(_) => Ok(Vec::new()),
    }
}

/// Poseedor, cantidad y sustantivo, en ese orden. El plural se pega al
/// sustantivo aunque haya número.
fn noun_phrase_words(conn: &Connection, rules: &Rules, np: &NounPhrase) -> Result<Vec<String>, GenError> {
    let mut out = Vec::new();
    if let Some(owner) = &np.possessor {
        out.push(word(conn, owner)?);
    }
    // Si la cantidad es lo que se pregunta, "cuántos" ya salió al inicio.
    if let Some(Quantity::Exact(n)) = np.quantity {
        out.push(number_words(conn, LANG, n)?);
    }
    let mut noun = word(conn, &np.noun)?;
    if np.is_plural() {
        noun.push_str(&rules.plural);
    }
    out.push(noun);
    Ok(out)
}

/// Verbo con la negación como prefijo y el tiempo como sufijo.
fn verb_form(conn: &Connection, rules: &Rules, sentence: &SemanticSentence) -> Result<String, GenError> {
    let mut verb = String::new();
    if sentence.negated {
        verb.push_str(&rules.negation);
    }
    verb.push_str(&word(conn, &sentence.verb)?);
    match sentence.tense {
        Tense::Past => verb.push_str(&rules.past),
        Tense::Future => verb.push_str(&rules.future),
        Tense::Present => {}
    }
    Ok(verb)
}
