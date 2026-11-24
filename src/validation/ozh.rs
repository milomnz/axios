//! Validador de Ozh: lee una frase ya escrita y comprueba que sea válida.
//!
//! Gramática que reconoce (orden verbo, sujeto, objeto):
//!
//! ```text
//! frase     := [ʔoka | venu] [ne] VERBO PARTÍCULA sintagma{0,2} [INTERROGATIVO]
//! sintagma  := PRONOMBRE | [NÚMERO] SUSTANTIVO [PRONOMBRE]
//! ```
//!
//! La partícula evidencial es obligatoria. El verbo lleva el prefijo de
//! pregunta si y solo si la frase es una pregunta. Sin interrogativo la frase
//! necesita sujeto. Un sustantivo seguido de pronombre se lee como poseído.

use super::vocabulary::{Category, Vocabulary};
use super::{Reading, Tense, Token, TokenKind, ValidationError, assemble, chunks, noun_argument, readings};
use crate::ir::Argument;
use crate::backends::Language;
use crate::backends::lookup;
use rusqlite::Connection;

const LANG: Language = Language::Ozh;

/// Lo que el validador necesita saber de la lengua, leído de la base.
struct Context {
    vocab: Vocabulary,
    past: String,
    future: String,
    negation: String,
    question: String,
    evidentials: Vec<String>,
    separator: String,
    base: u32,
}

impl Context {
    fn load(conn: &Connection) -> Result<Context, ValidationError> {
        let rule = |name: &'static str| -> Result<String, ValidationError> {
            lookup::rule_form(conn, LANG, name)?.ok_or(ValidationError::MissingRule(name))
        };
        let base = rule("NUMBER_BASE")?.parse().map_err(|_| ValidationError::MissingRule("NUMBER_BASE"))?;
        let evidentials = lookup::rule_forms(conn, LANG, "EVIDENTIAL")?;
        if evidentials.is_empty() {
            return Err(ValidationError::MissingRule("EVIDENTIAL"));
        }
        Ok(Context {
            vocab: Vocabulary::load(conn, LANG)?,
            past: rule("PAST")?,
            future: rule("FUTURE")?,
            negation: rule("NEGATION")?,
            question: rule("QUESTION")?,
            evidentials,
            separator: rule("NUMBER_SEPARATOR")?,
            base,
        })
    }
}

/// Lee `text` como Ozh. Devuelve lo entendido o el primer problema hallado.
pub fn validate(conn: &Connection, text: &str) -> Result<Reading, ValidationError> {
    let ctx = Context::load(conn)?;
    let tokens = lex(&ctx, text)?;
    let (is_question, mut irs) = parse(&tokens, text.chars().count())?;
    let ir = irs.remove(0);
    Ok(Reading { tokens, is_question, ir, alternatives: irs })
}

/// Analizador léxico: reconoce cada palabra completa. Las palabras de Ozh no
/// se parten, salvo el prefijo de pregunta del verbo.
fn lex(ctx: &Context, text: &str) -> Result<Vec<Token>, ValidationError> {
    chunks(text)
        .into_iter()
        .map(|(start, word)| {
            let kind = classify(ctx, word)
                .map_err(|why| ValidationError::invalid(start, format!("{word}: {why}")))?;
            Ok(Token { text: word.to_string(), start, kind })
        })
        .collect()
}

/// Dice que es una palabra, o por que no se reconoce.
fn classify(ctx: &Context, word: &str) -> Result<TokenKind, String> {
    if word == ctx.past {
        return Ok(TokenKind::TenseMarker(Tense::Past));
    }
    if word == ctx.future {
        return Ok(TokenKind::TenseMarker(Tense::Future));
    }
    if word == ctx.negation {
        return Ok(TokenKind::Negation);
    }
    if ctx.evidentials.iter().any(|e| e == word) {
        return Ok(TokenKind::Evidential(word.to_string()));
    }
    if word.contains(ctx.separator.as_str()) {
        return ctx.vocab.parse_number(word, &ctx.separator, ctx.base).map(TokenKind::Number);
    }
    if let Some(entry) = ctx.vocab.get(word) {
        return Ok(match entry.category {
            Category::Pronoun => TokenKind::Pronoun { key: entry.key.clone() },
            Category::Noun => TokenKind::Noun { key: entry.key.clone(), plural: false },
            Category::Interrogative => TokenKind::Interrogative { key: entry.key.clone() },
            Category::Verb => TokenKind::OzhVerb { key: entry.key.clone(), question: false },
            Category::Number => TokenKind::Number(ctx.vocab.parse_number(word, &ctx.separator, ctx.base)?),
        });
    }
    // Verbo con el prefijo de pregunta pegado (`ʔasur` = ʔa + sur).
    if let Some(stem) = word.strip_prefix(ctx.question.as_str()) {
        if let Some(entry) = ctx.vocab.get_as(stem, Category::Verb) {
            return Ok(TokenKind::OzhVerb { key: entry.key.clone(), question: true });
        }
    }
    Err("palabra desconocida, el manual no la respalda".to_string())
}

/// Analizador sintáctico: comprueba la forma de la frase. Devuelve si es
/// pregunta y los árboles con que se puede leer, el preferido primero.
fn parse(tokens: &[Token], end: usize) -> Result<(bool, Vec<crate::ir::Ir>), ValidationError> {
    let at = |i: usize| tokens.get(i).map_or(end, |t| t.start);
    let mut i = 0;
    let mut tense = Tense::Present;
    if let Some(TokenKind::TenseMarker(t)) = tokens.get(i).map(|t| &t.kind) {
        tense = *t;
        i += 1;
    }
    let negated = matches!(tokens.get(i).map(|t| &t.kind), Some(TokenKind::Negation));
    if negated {
        i += 1;
    }
    let (verb, question) = match tokens.get(i).map(|t| &t.kind) {
        Some(TokenKind::OzhVerb { key, question }) => (key.clone(), *question),
        _ => return Err(ValidationError::invalid(at(i), "aquí debía ir el verbo")),
    };
    i += 1;
    if !matches!(tokens.get(i).map(|t| &t.kind), Some(TokenKind::Evidential(_))) {
        return Err(ValidationError::invalid(at(i), "falta la partícula evidencial después del verbo"));
    }
    i += 1;

    let has_wh = matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::Interrogative { .. })) && tokens.len() > i;
    if has_wh && !question {
        return Err(ValidationError::invalid(tokens[tokens.len() - 1].start, "el interrogativo pide el prefijo de pregunta en el verbo"));
    }
    let zone_end = tokens.len() - usize::from(has_wh);
    let wh = match tokens.last().map(|t| &t.kind) {
        Some(TokenKind::Interrogative { key }) if has_wh => Some(key.clone()),
        _ => None,
    };
    let zone = &tokens[i..zone_end];
    let groupings = readings(|mask| noun_phrases(zone, mask, end), wh.as_deref(), at(i))?;
    let irs = groupings
        .into_iter()
        .map(|phrases| assemble(tense, negated, verb.clone(), phrases, wh.clone(), question))
        .collect();
    Ok((question, irs))
}

/// Lee los sintagmas de la zona entre la partícula y el interrogativo, y
/// comprueba la forma de cada uno. Un pronombre después de un sustantivo puede
/// ser su poseedor o un sintagma aparte: cada caso toma su decisión de un bit
/// de `mask`. Devuelve los sintagmas y cuántos casos ambiguos hubo.
fn noun_phrases(zone: &[Token], mask: u32, end: usize) -> Result<(Vec<Argument>, u32), ValidationError> {
    let mut phrases = Vec::new();
    let mut spots = 0;
    let mut i = 0;
    while i < zone.len() {
        if let TokenKind::Pronoun { key } = &zone[i].kind {
            phrases.push(Argument::Pronoun(key.clone()));
            i += 1;
            continue;
        }
        let mut number = None;
        if let TokenKind::Number(n) = zone[i].kind {
            number = Some(n);
            i += 1;
        }
        match zone.get(i).map(|t| &t.kind) {
            Some(TokenKind::Noun { key, .. }) => {
                i += 1;
                let owner = match zone.get(i).map(|t| &t.kind) {
                    Some(TokenKind::Pronoun { key }) => {
                        spots += 1;
                        if (mask >> (spots - 1)) & 1 == 1 {
                            i += 1;
                            Some(key.as_str())
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                // Ozh no marca plural: lo dice el número.
                phrases.push(noun_argument(key, number.is_some_and(|n| n != 1), number, owner));
            }
            _ => {
                let start = zone.get(i).map_or(end, |t| t.start);
                return Err(ValidationError::invalid(start, "aquí debía ir un sustantivo"));
            }
        }
    }
    Ok((phrases, spots))
}
