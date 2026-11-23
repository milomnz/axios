//! Orden `palabra`: lo que el sistema sabe de una palabra en las tres lenguas,
//! hoy o en una fecha concreta.

use crate::error::Result;
use rusqlite::{Connection, params};
use std::collections::HashSet;

/// Una version de una palabra en el lexico (una fila de `lexicon`).
#[derive(Debug, Clone)]
pub struct RowVersion {
    pub language: String,
    pub word: String,
    pub concept_key: String,
    pub meaning: String,
    pub note: Option<String>,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub source: String,
}

/// Un concepto con sus palabras en cada lengua en el momento consultado.
#[derive(Debug)]
pub struct ConceptView {
    pub key: String,
    pub category: String,
    pub value: Option<i64>,
    pub es: Vec<String>,
    pub kethari: Vec<String>,
    pub ozh: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug)]
pub struct SignalMatch {
    pub day: i64,
    pub voice: String,
    pub transcription: String,
    pub decoder_output: String,
    pub form: String,
}

#[derive(Debug)]
pub struct WordReport {
    pub term: String,
    /// Momento consultado; `None` significa "ahora".
    pub at: Option<String>,
    pub concepts: Vec<ConceptView>,
    /// Todas las versiones de la palabra, tambien las ya corregidas.
    pub history: Vec<RowVersion>,
    pub rules: Vec<String>,
    pub spanish_grammar: Vec<String>,
    pub signals: Vec<SignalMatch>,
}

impl WordReport {
    pub fn is_empty(&self) -> bool {
        self.history.is_empty() && self.rules.is_empty() && self.spanish_grammar.is_empty()
    }
}

/// Recoge todo lo que hay sobre `term`. Con `at` responde que significaba en ese momento.
pub fn word_report(conn: &Connection, term: &str, at: Option<&str>) -> Result<WordReport> {
    let term = term.trim().to_lowercase();
    let mut history = versions(conn, "l.word = ?1", &term)?;
    if history.is_empty() {
        // Sin coincidencia exacta se prueba sin tildes: "cuando" encuentra "cuando".
        history = versions(conn, "fold(l.word) = fold(?1)", &term)?;
    }

    let mut concepts = Vec::new();
    let mut seen = HashSet::new();
    for v in history.iter().filter(|v| valid_at(v, at)) {
        if seen.insert(v.concept_key.clone()) {
            concepts.push(concept_view(conn, &v.concept_key, at)?);
        }
    }

    let mut rules = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT language, rule, IFNULL(position, ''), IFNULL(note, '') FROM grammar_rules WHERE form = ?1 ORDER BY language, rule",
    )?;
    for row in stmt.query_map([&term], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?)))? {
        let (language, rule, position, note) = row?;
        rules.push(format!("{language}: regla {rule} {position} {note}").split_whitespace().collect::<Vec<_>>().join(" "));
    }

    let mut spanish_grammar = Vec::new();
    let mut stmt = conn.prepare("SELECT kind FROM es_function_words WHERE word = ?1")?;
    for kind in stmt.query_map([&term], |r| r.get::<_, String>(0))? {
        spanish_grammar.push(format!("palabra gramatical ({})", kind?));
    }
    let mut stmt = conn.prepare("SELECT infinitive, tense, person, number FROM es_irregular_forms WHERE form = ?1")?;
    for row in stmt.query_map([&term], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?, r.get::<_, String>(3)?)))? {
        let (inf, tense, person, number) = row?;
        spanish_grammar.push(format!("forma irregular de «{inf}»: {tense}, persona {person}, {number}"));
    }

    let alien: Vec<(String, String)> = history
        .iter()
        .filter(|v| v.language != "ES")
        .map(|v| (v.language.clone(), v.word.clone()))
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let signals = attestations(conn, &alien)?;

    Ok(WordReport { term, at: at.map(str::to_string), concepts, history, rules, spanish_grammar, signals })
}

fn valid_at(v: &RowVersion, at: Option<&str>) -> bool {
    match at {
        None => v.valid_to.is_none(),
        Some(t) => v.valid_from.as_str() <= t && v.valid_to.as_deref().is_none_or(|to| t < to),
    }
}

fn versions(conn: &Connection, condition: &str, term: &str) -> Result<Vec<RowVersion>> {
    let sql = format!(
        "SELECT l.language, l.word, c.semantic_key,
                IFNULL((SELECT group_concat(e.word, ', ') FROM lexicon e
                         WHERE e.concept_id = l.concept_id AND e.language = 'ES'
                           AND e.valid_from <= l.valid_from AND (e.valid_to IS NULL OR e.valid_to > l.valid_from)), ''),
                l.note, l.valid_from, l.valid_to,
                s.file_name || IFNULL(' (version ' || s.version || ')', '')
           FROM lexicon l
           JOIN concepts c ON c.id = l.concept_id
           JOIN sources s ON s.id = l.source_id
          WHERE {condition}
          ORDER BY l.language, l.valid_from, l.id"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([term], |r| {
        Ok(RowVersion {
            language: r.get(0)?,
            word: r.get(1)?,
            concept_key: r.get(2)?,
            meaning: r.get(3)?,
            note: r.get(4)?,
            valid_from: r.get(5)?,
            valid_to: r.get(6)?,
            source: r.get(7)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

fn concept_view(conn: &Connection, key: &str, at: Option<&str>) -> Result<ConceptView> {
    let (id, category, value): (i64, String, Option<i64>) =
        conn.query_row("SELECT id, category, value FROM concepts WHERE semantic_key = ?1", [key], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
    let mut view = ConceptView { key: key.into(), category, value, es: vec![], kethari: vec![], ozh: vec![], notes: vec![] };
    let mut stmt = conn.prepare(
        "SELECT language, word, note FROM lexicon
          WHERE concept_id = ?1
            AND CASE WHEN ?2 IS NULL THEN valid_to IS NULL
                     ELSE valid_from <= ?2 AND (valid_to IS NULL OR ?2 < valid_to) END
          ORDER BY id",
    )?;
    for row in stmt.query_map(params![id, at], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, Option<String>>(2)?)))? {
        let (language, word, note) = row?;
        match language.as_str() {
            "ES" => view.es.push(word),
            "KETHARI" => view.kethari.push(word),
            _ => view.ozh.push(word),
        }
        if let Some(n) = note.filter(|n| !view.notes.contains(n)) {
            view.notes.push(n);
        }
    }
    Ok(view)
}

/// Señales en las que aparece alguna de las palabras, sola o con los afijos
/// que la gramatica de su lengua tiene registrados (nu-kor-ok, ombra-esh, ʔa-sur).
fn attestations(conn: &Connection, words: &[(String, String)]) -> Result<Vec<SignalMatch>> {
    let mut found = Vec::new();
    if words.is_empty() {
        return Ok(found);
    }
    let mut affixes = conn.prepare("SELECT form FROM grammar_rules WHERE language = ?1 AND position = ?2")?;
    let mut known_words = conn.prepare("SELECT DISTINCT word FROM lexicon WHERE language = ?1")?;
    let mut signals = conn.prepare(
        "SELECT day, voice, transcription, IFNULL(decoder_output, '') FROM signals
          WHERE voice = ?1 AND transcription IS NOT NULL ORDER BY day, id",
    )?;
    for language in ["KETHARI", "OZH"] {
        let targets: HashSet<&str> = words.iter().filter(|(l, _)| l == language).map(|(_, w)| w.as_str()).collect();
        if targets.is_empty() {
            continue;
        }
        let prefixes: Vec<String> = affixes.query_map(params![language, "PREFIX"], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
        let suffixes: Vec<String> = affixes.query_map(params![language, "SUFFIX"], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
        let known: HashSet<String> = known_words.query_map([language], |r| r.get(0))?.collect::<rusqlite::Result<_>>()?;
        let rows = signals.query_map([language], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?)))?;
        for row in rows {
            let (day, voice, transcription, decoder_output) = row?;
            let forms: Vec<&str> = transcription
                .split(|c: char| c.is_whitespace() || c == '-')
                .filter(|t| !t.is_empty())
                .filter(|t| {
                    // Una palabra que ya esta en el lexico solo cuenta como ella misma:
                    // "iresh" es "ellos", no "ir" + "-esh".
                    if known.contains(*t) { targets.contains(t) } else { stems(t, &prefixes, &suffixes).iter().any(|s| targets.contains(s.as_str())) }
                })
                .collect();
            if !forms.is_empty() {
                found.push(SignalMatch { day, voice, transcription: transcription.clone(), decoder_output, form: forms.join(", ") });
            }
        }
    }
    found.sort_by_key(|m| m.day);
    Ok(found)
}

/// Raices posibles quitando a lo sumo un prefijo y dos sufijos.
fn stems(token: &str, prefixes: &[String], suffixes: &[String]) -> Vec<String> {
    let mut out = vec![token.to_string()];
    for p in prefixes {
        if let Some(rest) = token.strip_prefix(p.as_str()).filter(|r| !r.is_empty()) {
            out.push(rest.to_string());
        }
    }
    for _ in 0..2 {
        let mut next = Vec::new();
        for stem in &out {
            for s in suffixes {
                if let Some(rest) = stem.strip_suffix(s.as_str()).filter(|r| !r.is_empty()) {
                    next.push(rest.to_string());
                }
            }
        }
        out.extend(next);
    }
    out
}

/// Vocabulario español vigente para el corrector (`valid_words` del contrato
/// `SpanishLexicon` en `frontend/spellchecker.rs`): palabras del lexico vigentes,
/// palabras gramaticales y formas verbales irregulares, sin repetir y en orden.
/// Los significados de varias palabras ("territorio reclamado") se separan.
pub fn spanish_words(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT word FROM lexicon WHERE language = 'ES' AND valid_to IS NULL
         UNION SELECT word FROM es_function_words
         UNION SELECT form FROM es_irregular_forms",
    )?;
    let mut words = std::collections::BTreeSet::new();
    for entry in stmt.query_map([], |r| r.get::<_, String>(0))? {
        words.extend(entry?.split_whitespace().map(str::to_string));
    }
    Ok(words.into_iter().collect())
}

/// `exists` del contrato `SpanishLexicon`: si la palabra esta tal cual en el vocabulario vigente.
pub fn spanish_word_exists(conn: &Connection, word: &str) -> Result<bool> {
    Ok(spanish_words(conn)?.iter().any(|w| w == word))
}

#[cfg(test)]
mod tests {
    use super::stems;

    #[test]
    fn quita_afijos() {
        let pre = vec!["nu".to_string()];
        let suf = vec!["ok".to_string(), "ime".to_string(), "esh".to_string()];
        assert!(stems("nukorok", &pre, &suf).contains(&"kor".to_string()));
        assert!(stems("ombraesh", &pre, &suf).contains(&"ombra".to_string()));
        assert!(!stems("anir", &pre, &suf).contains(&"an".to_string()));
    }
}
