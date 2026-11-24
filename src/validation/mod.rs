//! Validadores de Kethari y Ozh.
//!
//! Antes de transmitir una traducción, cada validador la lee de nuevo con su
//! propio analizador léxico y sintáctico, sin usar nada de los generadores. Si
//! la frase no es válida (palabra que el manual no respalda, orden incorrecto,
//! partícula que falta), no sale.

pub mod kethari;
pub mod ozh;
mod vocabulary;

use crate::ir::{Argument, Ir, NounPhrase, Quantity, SemanticQuestion, SemanticSentence};
use std::fmt;

/// Que es un token ya reconocido. Las claves (`key`) son las `semantic_key` de
/// la tabla `concepts`, para poder volver a llevar la frase al español.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Pronoun { key: String },
    Noun { key: String, plural: bool },
    Number(u32),
    Interrogative { key: String },
    /// Verbo de Kethari: la negación es prefijo y el tiempo sufijo.
    KethariVerb { key: String, negated: bool, tense: Tense },
    /// Verbo de Ozh: la pregunta se marca con un prefijo.
    OzhVerb { key: String, question: bool },
    /// Partícula `ka` que cierra las preguntas de Kethari.
    QuestionParticle,
    /// Partícula evidencial de Ozh, justo después del verbo.
    Evidential(String),
    /// `ʔoka` o `venu` al inicio de una oración de Ozh.
    TenseMarker(Tense),
    /// `ne`, antes del verbo de Ozh.
    Negation,
}

/// Tiempo con que se lee un verbo.
pub use crate::ir::Tense;

/// Una palabra de la frase con su lugar: `start` cuenta caracteres desde cero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub text: String,
    pub start: usize,
    pub kind: TokenKind,
}

/// Lo que el validador entiende de una frase válida: sus palabras y el árbol
/// que la describe, el mismo que usan los generadores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reading {
    pub tokens: Vec<Token>,
    pub is_question: bool,
    /// La lectura preferida de la frase.
    pub ir: Ir,
    /// Otras lecturas igual de válidas. Una frase puede ser ambigua: en
    /// Kethari, `anul parn` es "nosotros, paz" o "nuestra paz".
    pub alternatives: Vec<Ir>,
}

#[derive(Debug)]
pub enum ValidationError {
    /// La frase no es válida; `position` es el carácter donde se nota el problema.
    Invalid { position: usize, message: String },
    /// La base no tiene las reglas que el validador necesita para leer la lengua.
    MissingRule(&'static str),
    Db(rusqlite::Error),
}

impl ValidationError {
    pub(crate) fn invalid(position: usize, message: impl Into<String>) -> Self {
        ValidationError::Invalid { position, message: message.into() }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::Invalid { position, message } => write!(f, "posición {position}: {message}"),
            ValidationError::MissingRule(rule) => write!(f, "falta la regla {rule} en la base"),
            ValidationError::Db(e) => write!(f, "error de base de datos: {e}"),
        }
    }
}

impl std::error::Error for ValidationError {}

impl From<rusqlite::Error> for ValidationError {
    fn from(e: rusqlite::Error) -> Self {
        ValidationError::Db(e)
    }
}

/// Parte la frase en palabras separadas por espacios y da el carácter donde
/// empieza cada una.
pub(crate) fn chunks(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start: Option<(usize, usize)> = None;
    let mut count = 0;
    for (byte, ch) in text.char_indices() {
        match (ch.is_whitespace(), start) {
            (false, None) => start = Some((count, byte)),
            (true, Some((first_char, first_byte))) => {
                out.push((first_char, &text[first_byte..byte]));
                start = None;
            }
            _ => {}
        }
        count += 1;
    }
    if let Some((first_char, first_byte)) = start {
        out.push((first_char, &text[first_byte..]));
    }
    out
}

/// Arma el árbol de una frase ya validada a partir de sus sintagmas, en el orden
/// en que aparecieron (sujeto y después objeto). `wh` es la clave del
/// interrogativo, si lo hay.
///
/// Con un solo sintagma y un interrogativo de sujeto u objeto, la frase no dice
/// cuál de los dos falta. Se asume que un pronombre solo es el sujeto y que un
/// sustantivo solo es el objeto, como en "qué quieren ustedes" frente a "quién
/// envio el mensaje".
pub(crate) fn assemble(
    tense: Tense,
    negated: bool,
    verb: String,
    mut phrases: Vec<Argument>,
    wh: Option<String>,
    is_question: bool,
) -> Ir {
    let mut adjunct = None;
    let mut wh_argument = None;
    match wh.as_deref() {
        Some("CUANDO") | Some("DONDE") => adjunct = wh.clone(),
        Some("CUANTOS") => ask_quantity(&mut phrases),
        Some(key) => wh_argument = Some(Argument::Wh(key.to_string())),
        None => {}
    }
    let mut phrases = phrases.into_iter();
    let (subject, object) = match wh_argument {
        None => (phrases.next(), phrases.next()),
        Some(w) => match phrases.next() {
            None => (Some(w), None),
            Some(only @ Argument::Pronoun(_)) => (Some(only), Some(w)),
            Some(only) => (Some(w), Some(only)),
        },
    };
    let sentence = SemanticSentence { tense, negated, verb, subject, object };
    if is_question {
        Ir::Question(SemanticQuestion { sentence, adjunct })
    } else {
        Ir::Sentence(sentence)
    }
}

/// "Cuántos": la cantidad se pregunta en el primer sustantivo que no trae número.
fn ask_quantity(phrases: &mut [Argument]) {
    let target = phrases.iter_mut().find_map(|a| match a {
        Argument::Noun(np) if np.quantity.is_none() => Some(np),
        _ => None,
    });
    if let Some(np) = target {
        np.quantity = Some(Quantity::Unknown);
        np.plural = true;
    }
}

/// Un sustantivo ya leído, con su cantidad y poseedor, como sintagma del árbol.
pub(crate) fn noun_argument(noun: &str, plural: bool, number: Option<u32>, owner: Option<&str>) -> Argument {
    let mut np = NounPhrase::new(noun);
    np.plural = plural;
    np.quantity = number.map(Quantity::Exact);
    np.possessor = owner.map(str::to_string);
    Argument::Noun(np)
}

/// Todas las maneras válidas de agrupar los sintagmas de una frase.
///
/// `read(mask)` agrupa la zona de sintagmas; cada lugar ambiguo toma su
/// decisión de un bit de `mask` y la función devuelve también cuántos lugares
/// ambiguos encontró. La lectura sin fusiones va primero. Con un interrogativo
/// de sujeto u objeto cabe un solo sintagma; con "cuándo", "dónde" o "cuántos",
/// dos; sin interrogativo hace falta al menos el sujeto.
pub(crate) fn readings<F>(read: F, wh: Option<&str>, position: usize) -> Result<Vec<Vec<Argument>>, ValidationError>
where
    F: Fn(u32) -> Result<(Vec<Argument>, u32), ValidationError>,
{
    let room = if matches!(wh, None | Some("CUANDO" | "DONDE" | "CUANTOS")) { 2 } else { 1 };
    let (first, spots) = read(0)?;
    let mut found = Vec::new();
    for mask in 0..(1u32 << spots.min(4)) {
        let phrases = if mask == 0 { first.clone() } else { read(mask)?.0 };
        if phrases.len() <= room && !(phrases.is_empty() && wh.is_none()) {
            found.push(phrases);
        }
    }
    if found.is_empty() {
        let why = if first.len() > room { "hay más sintagmas de los que la frase admite" } else { "falta el sujeto" };
        return Err(ValidationError::invalid(position, why));
    }
    Ok(found)
}
