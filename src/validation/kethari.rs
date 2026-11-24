//! Validador de Kethari: lee una frase ya escrita y comprueba que sea válida.
//!
//! Gramática que reconoce (orden sujeto, objeto, verbo):
//!
//! ```text
//! frase     := [INTERROGATIVO] sintagma{0,2} VERBO [ka]
//! sintagma  := PRONOMBRE | [PRONOMBRE] [NÚMERO] SUSTANTIVO
//! ```
//!
//! Sin interrogativo la frase necesita sujeto. Si lleva interrogativo, este
//! puede ocupar el lugar del sujeto o del objeto. `ka` va al final y solo en
//! preguntas. Un número distinto de uno pide sustantivo en plural.

use super::vocabulary::{Category, Vocabulary};
use super::{Reading, Tense, Token, TokenKind, ValidationError, assemble, chunks, noun_argument, readings};
use crate::ir::Argument;
use crate::backends::Language;
use rusqlite::Connection;

const LANG: Language = Language::Kethari;

/// Lo que el validador necesita saber de la lengua, leído de la base.
struct Context {
    vocab: Vocabulary,
    past: String,
    future: String,
    negation: String,
    plural: String,
    question: String,
    separator: String,
    base: u32,
}

impl Context {
    fn load(conn: &Connection) -> Result<Context, ValidationError> {
        let rule = |name: &'static str| -> Result<String, ValidationError> {
            crate::backends::lookup::rule_form(conn, LANG, name)?.ok_or(ValidationError::MissingRule(name))
        };
        let base = rule("NUMBER_BASE")?.parse().map_err(|_| ValidationError::MissingRule("NUMBER_BASE"))?;
        Ok(Context {
            vocab: Vocabulary::load(conn, LANG)?,
            past: rule("PAST")?,
            future: rule("FUTURE")?,
            negation: rule("NEGATION")?,
            plural: rule("PLURAL")?,
            question: rule("QUESTION")?,
            separator: rule("NUMBER_SEPARATOR")?,
            base,
        })
    }
}

/// Lee `text` como Kethari. Devuelve lo entendido o el primer problema hallado.
pub fn validate(conn: &Connection, text: &str) -> Result<Reading, ValidationError> {
    let ctx = Context::load(conn)?;
    let tokens = lex(&ctx, text)?;
    let (is_question, mut irs) = parse(&tokens, text.chars().count())?;
    let ir = irs.remove(0);
    Ok(Reading { tokens, is_question, ir, alternatives: irs })
}

/// Analizador léxico: reconoce cada palabra completa, con sus afijos.
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
    if word == ctx.question {
        return Ok(TokenKind::QuestionParticle);
    }
    if word.contains(ctx.separator.as_str()) {
        return ctx.vocab.parse_number(word, &ctx.separator, ctx.base).map(TokenKind::Number);
    }
    if let Some(entry) = ctx.vocab.get(word) {
        return Ok(match entry.category {
            Category::Pronoun => TokenKind::Pronoun { key: entry.key.clone() },
            Category::Noun => TokenKind::Noun { key: entry.key.clone(), plural: false },
            Category::Interrogative => TokenKind::Interrogative { key: entry.key.clone() },
            Category::Verb => TokenKind::KethariVerb { key: entry.key.clone(), negated: false, tense: Tense::Present },
            Category::Number => TokenKind::Number(ctx.vocab.parse_number(word, &ctx.separator, ctx.base)?),
        });
    }
    if let Some(stem) = word.strip_suffix(ctx.plural.as_str()) {
        if let Some(entry) = ctx.vocab.get_as(stem, Category::Noun) {
            return Ok(TokenKind::Noun { key: entry.key.clone(), plural: true });
        }
    }
    verb_with_affixes(ctx, word).ok_or_else(|| "palabra desconocida, el manual no la respalda".to_string())
}

/// Verbo con negación como prefijo y tiempo como sufijo (`nukorok` = nu + kor + ok).
fn verb_with_affixes(ctx: &Context, word: &str) -> Option<TokenKind> {
    let (negated, rest) = match word.strip_prefix(ctx.negation.as_str()) {
        Some(rest) => (true, rest),
        None => (false, word),
    };
    let (tense, stem) = if let Some(stem) = rest.strip_suffix(ctx.past.as_str()) {
        (Tense::Past, stem)
    } else if let Some(stem) = rest.strip_suffix(ctx.future.as_str()) {
        (Tense::Future, stem)
    } else {
        (Tense::Present, rest)
    };
    let entry = ctx.vocab.get_as(stem, Category::Verb)?;
    Some(TokenKind::KethariVerb { key: entry.key.clone(), negated, tense })
}

/// Analizador sintáctico: comprueba la forma de la frase. Devuelve si es
/// pregunta y los árboles con que se puede leer, el preferido primero.
fn parse(tokens: &[Token], end: usize) -> Result<(bool, Vec<crate::ir::Ir>), ValidationError> {
    let at = |i: usize| tokens.get(i).map_or(end, |t| t.start);
    if tokens.is_empty() {
        return Err(ValidationError::invalid(0, "la frase está vacía"));
    }

    let has_wh = matches!(tokens[0].kind, TokenKind::Interrogative { .. });
    let has_ka = matches!(tokens.last().map(|t| &t.kind), Some(TokenKind::QuestionParticle));
    if has_wh && !has_ka {
        return Err(ValidationError::invalid(end, "una pregunta con interrogativo debe terminar en ka"));
    }

    let first = usize::from(has_wh);
    let last = tokens.len() - usize::from(has_ka);
    if last <= first {
        return Err(ValidationError::invalid(end, "falta el verbo"));
    }
    if !matches!(tokens[last - 1].kind, TokenKind::KethariVerb { .. }) {
        return Err(ValidationError::invalid(at(last - 1), "la frase debe cerrar con el verbo"));
    }

    let wh = match &tokens[0].kind {
        TokenKind::Interrogative { key } => Some(key.clone()),
        _ => None,
    };
    let zone = &tokens[first..last - 1];
    let groupings = readings(|mask| noun_phrases(zone, mask), wh.as_deref(), at(first))?;
    let TokenKind::KethariVerb { key, negated, tense } = &tokens[last - 1].kind else { unreachable!() };
    let irs = groupings
        .into_iter()
        .map(|phrases| assemble(*tense, *negated, key.clone(), phrases, wh.clone(), has_ka))
        .collect();
    Ok((has_ka, irs))
}

/// Lee los sintagmas de la zona entre el interrogativo y el verbo y comprueba
/// la forma de cada uno. Un pronombre seguido de número o sustantivo puede ser
/// su poseedor o un sintagma aparte: cada caso toma su decisión de un bit de
/// `mask`. Devuelve los sintagmas y cuántos casos ambiguos hubo.
fn noun_phrases(zone: &[Token], mask: u32) -> Result<(Vec<Argument>, u32), ValidationError> {
    let mut phrases = Vec::new();
    let mut spots = 0;
    let mut i = 0;
    while i < zone.len() {
        let mut owner = None;
        if let TokenKind::Pronoun { key } = &zone[i].kind {
            let may_own = matches!(zone.get(i + 1).map(|t| &t.kind), Some(TokenKind::Number(_) | TokenKind::Noun { .. }));
            if may_own {
                spots += 1;
            }
            if may_own && (mask >> (spots - 1)) & 1 == 1 {
                owner = Some(key.as_str());
                i += 1;
            } else {
                phrases.push(Argument::Pronoun(key.clone()));
                i += 1;
                continue;
            }
        }
        let mut number = None;
        if let Some(TokenKind::Number(n)) = zone.get(i).map(|t| &t.kind) {
            number = Some(*n);
            i += 1;
        }
        match zone.get(i).map(|t| &t.kind) {
            Some(TokenKind::Noun { key, plural }) => {
                if let Some(n) = number {
                    if (n == 1) == *plural {
                        let why = if n == 1 { "con el número uno el sustantivo va en singular" } else { "con un número mayor que uno el sustantivo lleva plural" };
                        return Err(ValidationError::invalid(zone[i].start, why));
                    }
                }
                phrases.push(noun_argument(key, *plural, number, owner));
                i += 1;
            }
            _ => {
                let start = zone.get(i).or(zone.last()).map_or(0, |t| t.start);
                return Err(ValidationError::invalid(start, "aquí debía ir un sustantivo"));
            }
        }
    }
    Ok((phrases, spots))
}
