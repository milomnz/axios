//! Orden `cargar`: incorpora un archivo de datos sin recompilar.
//!
//! Cada linea empieza por su tipo y los campos van separados por `|`.
//! Las lineas vacias y las que empiezan por `#` se ignoran.
//!
//! | Tipo        | Campos                                                   |
//! |-------------|----------------------------------------------------------|
//! | LEXICO      | categoria, clave, español, kethari, ozh, nota            |
//! | NUMERO      | valor, kethari, ozh                                      |
//! | REGLA       | lengua, regla, posicion, forma, nota                     |
//! | DEDUCCION   | lengua, regla, posicion, forma, evidencia                |
//! | FORMA_ES    | clave, palabra, nota                                     |
//! | FUNCIONAL   | palabra, tipo                                            |
//! | NUMERAL_ES  | palabra, valor                                           |
//! | DESINENCIA  | conjugacion, tiempo, persona, numero, terminacion, base  |
//! | IRREGULAR   | forma, infinitivo, tiempo, persona, numero               |
//! | SENAL       | dia, voz, transcripcion, salida del decodificador        |
//! | CORRECCION  | lengua, palabra, significado anterior, significado nuevo |
//!
//! Todo el archivo se carga en una sola transaccion: si una linea falla no queda
//! nada a medias. Cargar dos veces el mismo archivo no duplica datos.

use super::{concept_key, fold, language_code, now};
use crate::error::{AxiosError, Result};
use rusqlite::{Connection, OptionalExtension, Transaction, params};
use std::fmt;
use std::path::Path;

#[derive(Debug, Default, PartialEq)]
pub struct LoadReport {
    pub kind: String,
    pub version: Option<String>,
    pub words_added: usize,
    pub rules_added: usize,
    pub spanish_added: usize,
    pub signals_added: usize,
    pub signals_repeated: usize,
    pub corrections_applied: usize,
    pub corrections_repeated: usize,
}

impl fmt::Display for LoadReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let version = self.version.as_deref().map(|v| format!(" (version {v})")).unwrap_or_default();
        writeln!(f, "Archivo de tipo {}{version}", self.kind)?;
        let lines = [
            ("palabras nuevas en el lexico", self.words_added),
            ("reglas gramaticales nuevas", self.rules_added),
            ("datos nuevos del español", self.spanish_added),
            ("señales nuevas", self.signals_added),
            ("señales que ya estaban registradas", self.signals_repeated),
            ("correcciones aplicadas", self.corrections_applied),
            ("correcciones que ya estaban aplicadas", self.corrections_repeated),
        ];
        for (label, n) in lines.iter().filter(|(_, n)| *n > 0) {
            writeln!(f, "  {n:>3} {label}")?;
        }
        if lines.iter().all(|(_, n)| *n == 0) {
            writeln!(f, "  No habia nada nuevo: la base ya tenia todo lo del archivo.")?;
        }
        Ok(())
    }
}

pub fn load_file(conn: &mut Connection, path: &Path) -> Result<LoadReport> {
    let file = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let text = std::fs::read_to_string(path).map_err(|source| AxiosError::Io { path: path.display().to_string(), source })?;
    let kind = classify(&text).ok_or_else(|| AxiosError::Load {
        file: file.clone(),
        line: 0,
        message: "no contiene ninguna linea de datos reconocible".into(),
    })?;
    let mut report = LoadReport { kind: kind.into(), version: header_version(&text), ..Default::default() };

    let tx = conn.transaction()?;
    let loaded_at = now(&tx)?;
    tx.execute(
        "INSERT INTO sources (kind, file_name, version, loaded_at) VALUES (?1, ?2, ?3, ?4)",
        params![kind, file, report.version, loaded_at],
    )?;
    let mut ctx = Ctx { tx: &tx, source: tx.last_insert_rowid(), now: loaded_at };

    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('|').map(str::trim).collect();
        ctx.line(&fields, &mut report)
            .map_err(|message| AxiosError::Load { file: file.clone(), line: i + 1, message })?;
    }
    tx.commit()?;
    Ok(report)
}

/// Tipo de fuente segun las lineas que contiene.
fn classify(text: &str) -> Option<&'static str> {
    let types: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| l.split('|').next())
        .collect();
    let has = |names: &[&str]| types.iter().any(|t| names.contains(t));
    if has(&["SENAL", "CORRECCION"]) {
        Some("SIGNALS")
    } else if has(&["DEDUCCION"]) {
        Some("TEAM")
    } else if has(&["FORMA_ES", "FUNCIONAL", "NUMERAL_ES", "DESINENCIA", "IRREGULAR"]) {
        Some("SPANISH")
    } else if has(&["LEXICO", "NUMERO", "REGLA"]) {
        Some("MANUAL")
    } else {
        None
    }
}

/// Version del manual anunciada en los comentarios de cabecera ("manual version 0.8").
fn header_version(text: &str) -> Option<String> {
    text.lines().take_while(|l| l.trim_start().starts_with('#')).find_map(|l| {
        // Los archivos de Axios escriben la palabra con tilde; fold() la quita.
        let lower = fold(l);
        let pos = lower.find("version")? + "version".len();
        lower[pos..].split_whitespace().next().map(str::to_string)
    })
}

type LineResult = std::result::Result<(), String>;

struct Ctx<'a> {
    tx: &'a Transaction<'a>,
    source: i64,
    now: String,
}

fn db(e: rusqlite::Error) -> String {
    e.to_string()
}

/// Comprueba la cantidad de campos (contando el tipo) y rellena los opcionales con "".
fn take<'a>(fields: &[&'a str], min: usize, max: usize) -> std::result::Result<Vec<&'a str>, String> {
    if fields.len() < min || fields.len() > max {
        let expected = if min == max { min.to_string() } else { format!("entre {min} y {max}") };
        return Err(format!("{} necesita {expected} campos y tiene {}", fields[0], fields.len()));
    }
    let mut v = fields.to_vec();
    v.resize(max, "");
    Ok(v)
}

fn lang(name: &str) -> std::result::Result<&'static str, String> {
    language_code(name).ok_or_else(|| format!("lengua desconocida «{name}»"))
}

fn number(text: &str, what: &str) -> std::result::Result<i64, String> {
    text.parse().map_err(|_| format!("{what} «{text}» no es un numero"))
}

fn words(list: &str) -> impl Iterator<Item = String> + '_ {
    list.split(',').map(|w| w.trim().to_lowercase()).filter(|w| !w.is_empty())
}

impl Ctx<'_> {
    fn line(&mut self, f: &[&str], r: &mut LoadReport) -> LineResult {
        match f[0] {
            "LEXICO" => self.lexico(&take(f, 6, 7)?, r),
            "NUMERO" => self.numero(&take(f, 4, 4)?, r),
            "REGLA" | "DEDUCCION" => self.regla(&take(f, 5, 6)?, r),
            "FORMA_ES" => self.forma_es(&take(f, 3, 4)?, r),
            "FUNCIONAL" => {
                let f = take(f, 3, 3)?;
                let n = self.tx.execute("INSERT OR IGNORE INTO es_function_words (word, kind) VALUES (?1, ?2)", params![f[1].to_lowercase(), f[2]]).map_err(db)?;
                r.spanish_added += n;
                Ok(())
            }
            "NUMERAL_ES" => {
                let f = take(f, 3, 3)?;
                let value = number(f[2], "el valor")?;
                if !(0..=9999).contains(&value) {
                    return Err(format!("el valor {value} esta fuera de 0..9999"));
                }
                let concept = self.concept("NUMBER", &format!("NUM_{value}"), Some(value))?;
                r.spanish_added += self.add_word(concept, "ES", &f[1].to_lowercase(), "")? as usize;
                Ok(())
            }
            "DESINENCIA" => {
                let f = take(f, 7, 7)?;
                let n = self
                    .tx
                    .execute(
                        "INSERT OR IGNORE INTO es_verb_endings (conjugation, tense, person, number, ending, attaches_to)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                        params![f[1], f[2], number(f[3], "la persona")?, f[4], f[5], f[6]],
                    )
                    .map_err(db)?;
                r.spanish_added += n;
                Ok(())
            }
            "IRREGULAR" => {
                let f = take(f, 6, 6)?;
                let n = self
                    .tx
                    .execute(
                        "INSERT OR IGNORE INTO es_irregular_forms (form, infinitive, tense, person, number) VALUES (?1, ?2, ?3, ?4, ?5)",
                        params![f[1].to_lowercase(), f[2].to_lowercase(), f[3], number(f[4], "la persona")?, f[5]],
                    )
                    .map_err(db)?;
                r.spanish_added += n;
                Ok(())
            }
            "SENAL" => self.senal(&take(f, 5, 5)?, r),
            "CORRECCION" => self.correccion(&take(f, 5, 5)?, r),
            other => Err(format!("tipo de linea desconocido «{other}»")),
        }
    }

    /// Devuelve el id del concepto, creandolo si no existe.
    fn concept(&self, category: &str, key: &str, value: Option<i64>) -> std::result::Result<i64, String> {
        let found: Option<(i64, String)> = self
            .tx
            .query_row("SELECT id, category FROM concepts WHERE semantic_key = ?1", [key], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()
            .map_err(db)?;
        match found {
            Some((id, cat)) if cat == category => Ok(id),
            Some((_, cat)) => Err(format!("el concepto {key} ya existe con categoria {cat}, no {category}")),
            None => {
                self.tx
                    .execute("INSERT INTO concepts (category, semantic_key, value) VALUES (?1, ?2, ?3)", params![category, key, value])
                    .map_err(db)?;
                Ok(self.tx.last_insert_rowid())
            }
        }
    }

    /// Agrega una palabra vigente desde ahora. Si esa misma asociacion palabra-concepto
    /// existio alguna vez (aunque hoy este cerrada por una correccion) no se vuelve a abrir:
    /// asi recargar el manual viejo no deshace las correcciones.
    fn add_word(&self, concept: i64, language: &str, word: &str, note: &str) -> std::result::Result<bool, String> {
        let known: bool = self
            .tx
            .query_row(
                "SELECT EXISTS (SELECT 1 FROM lexicon WHERE concept_id = ?1 AND language = ?2 AND word = ?3)",
                params![concept, language, word],
                |r| r.get(0),
            )
            .map_err(db)?;
        if known {
            return Ok(false);
        }
        self.insert_word(concept, language, word, note)?;
        Ok(true)
    }

    fn insert_word(&self, concept: i64, language: &str, word: &str, note: &str) -> std::result::Result<i64, String> {
        let note = (!note.is_empty()).then_some(note);
        self.tx
            .execute(
                "INSERT INTO lexicon (concept_id, language, word, note, source_id, valid_from) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![concept, language, word, note, self.source, self.now],
            )
            .map_err(db)?;
        Ok(self.tx.last_insert_rowid())
    }

    fn lexico(&self, f: &[&str], r: &mut LoadReport) -> LineResult {
        let concept = self.concept(f[1], f[2], None)?;
        for (language, list) in [("ES", f[3]), ("KETHARI", f[4]), ("OZH", f[5])] {
            for word in words(list) {
                r.words_added += self.add_word(concept, language, &word, f[6])? as usize;
            }
        }
        Ok(())
    }

    fn numero(&self, f: &[&str], r: &mut LoadReport) -> LineResult {
        let value = number(f[1], "el valor")?;
        let concept = self.concept("NUMBER", &format!("NUM_{value}"), Some(value))?;
        for (language, list) in [("KETHARI", f[2]), ("OZH", f[3])] {
            for word in words(list) {
                r.words_added += self.add_word(concept, language, &word, "")? as usize;
            }
        }
        Ok(())
    }

    fn regla(&self, f: &[&str], r: &mut LoadReport) -> LineResult {
        let language = lang(f[1])?;
        if language == "ES" {
            return Err("las reglas gramaticales son solo de Kethari u Ozh".into());
        }
        let position = (!f[3].is_empty()).then_some(f[3]);
        let note = (!f[5].is_empty()).then_some(f[5]);
        r.rules_added += self
            .tx
            .execute(
                "INSERT OR IGNORE INTO grammar_rules (language, rule, position, form, note, source_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![language, f[2], position, f[4], note, self.source],
            )
            .map_err(db)?;
        Ok(())
    }

    fn forma_es(&self, f: &[&str], r: &mut LoadReport) -> LineResult {
        let concept: i64 = self
            .tx
            .query_row("SELECT id FROM concepts WHERE semantic_key = ?1", [f[1]], |r| r.get(0))
            .optional()
            .map_err(db)?
            .ok_or_else(|| format!("el concepto {} no existe; cargue primero el manual", f[1]))?;
        r.spanish_added += self.add_word(concept, "ES", &f[2].to_lowercase(), f[3])? as usize;
        Ok(())
    }

    fn senal(&self, f: &[&str], r: &mut LoadReport) -> LineResult {
        let day = number(f[1], "el dia")?;
        let voice = match f[2].to_lowercase().as_str() {
            "kethari" => "KETHARI",
            "ozh" => "OZH",
            "desconocida" => "UNKNOWN",
            other => return Err(format!("voz desconocida «{other}»")),
        };
        let transcription = (!f[3].is_empty()).then_some(f[3]);
        let output = (!f[4].is_empty()).then_some(f[4]);
        let n = self
            .tx
            .execute(
                "INSERT OR IGNORE INTO signals (day, voice, transcription, decoder_output, source_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![day, voice, transcription, output, self.source],
            )
            .map_err(db)?;
        if n == 1 { r.signals_added += 1 } else { r.signals_repeated += 1 }
        Ok(())
    }

    /// Una correccion cambia el significado de una palabra alienigena. No se borra nada:
    /// la fila vieja se cierra en `now` y se abre una nueva con el concepto corregido.
    fn correccion(&self, f: &[&str], r: &mut LoadReport) -> LineResult {
        let language = lang(f[1])?;
        if language == "ES" {
            return Err("solo se corrigen palabras de Kethari u Ozh".into());
        }
        let word = f[2].to_lowercase();
        let (old, new) = (f[3], f[4]);
        if old.is_empty() || new.is_empty() {
            return Err("faltan el significado anterior o el nuevo".into());
        }

        // Filas vigentes de la palabra, con los significados en español de su concepto.
        let mut stmt = self
            .tx
            .prepare(
                "SELECT l.id, l.concept_id, c.category,
                        (SELECT group_concat(e.word, '|') FROM lexicon e
                          WHERE e.concept_id = l.concept_id AND e.language = 'ES' AND e.valid_to IS NULL)
                   FROM lexicon l JOIN concepts c ON c.id = l.concept_id
                  WHERE l.language = ?1 AND l.word = ?2 AND l.valid_to IS NULL",
            )
            .map_err(db)?;
        let current: Vec<(i64, i64, String, String)> = stmt
            .query_map(params![language, word], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get::<_, Option<String>>(3)?.unwrap_or_default())))
            .and_then(|rows| rows.collect())
            .map_err(db)?;
        if current.is_empty() {
            return Err(format!("«{word}» no tiene ningun significado vigente en {language}"));
        }
        let means = |meanings: &str, m: &str| meanings.split('|').any(|x| fold(x) == fold(m));

        if current.iter().any(|(_, _, _, meanings)| means(meanings, new)) {
            r.corrections_repeated += 1;
            return Ok(());
        }
        let Some((old_id, _, category, _)) = current.iter().find(|(_, _, _, meanings)| means(meanings, old)) else {
            let actual: Vec<&str> = current.iter().map(|(_, _, _, m)| m.as_str()).collect();
            return Err(format!("«{word}» no significa «{old}» ahora mismo, significa «{}»", actual.join(", ").replace('|', ", ")));
        };
        if category == "NUMBER" {
            return Err("las correcciones de numeros no estan soportadas".into());
        }

        let concept = self.concept(category, &concept_key(new), None)?;
        self.add_word(concept, "ES", &new.to_lowercase(), "")?;
        self.tx.execute("UPDATE lexicon SET valid_to = ?1 WHERE id = ?2", params![self.now, old_id]).map_err(db)?;
        let new_id = self.insert_word(concept, language, &word, &format!("Corregido: antes «{old}»"))?;
        self.tx
            .execute(
                "INSERT INTO corrections (language, word, old_meaning, new_meaning, old_lexicon_id, new_lexicon_id, source_id, applied_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![language, word, old, new, old_id, new_id, self.source, self.now],
            )
            .map_err(db)?;
        r.corrections_applied += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_de_cabecera() {
        assert_eq!(header_version("# senales · manual version 0.8\nSENAL|1|ozh|a|b").as_deref(), Some("0.8"));
        assert_eq!(header_version("SENAL|1|ozh|a|b"), None);
    }

    #[test]
    fn clasifica_archivos() {
        assert_eq!(classify("SENAL|1|ozh|a|b\nCORRECCION|ozh|a|b|c"), Some("SIGNALS"));
        assert_eq!(classify("# solo comentarios"), None);
        assert_eq!(classify("LEXICO|NOUN|X|x|x|x|"), Some("MANUAL"));
    }
}
