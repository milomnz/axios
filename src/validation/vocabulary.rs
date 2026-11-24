//! Vocabulario vigente de una lengua, cargado de la base, y lectura de números.

use crate::backends::Language;
use rusqlite::Connection;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Noun,
    Verb,
    Pronoun,
    Interrogative,
    Number,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub category: Category,
    pub key: String,
    /// Valor de la cifra, solo para los números.
    pub value: Option<u32>,
}

/// Las palabras que el manual respalda hoy en una lengua.
pub struct Vocabulary {
    words: HashMap<String, Entry>,
}

impl Vocabulary {
    pub fn load(conn: &Connection, language: Language) -> rusqlite::Result<Vocabulary> {
        let mut stmt = conn.prepare(
            "SELECT l.word, c.category, c.semantic_key, c.value
               FROM lexicon l JOIN concepts c ON c.id = l.concept_id
              WHERE l.language = ?1 AND l.valid_to IS NULL ORDER BY l.id",
        )?;
        let rows = stmt.query_map([language.db_name()], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, Option<u32>>(3)?))
        })?;
        let mut words = HashMap::new();
        for row in rows {
            let (word, category, key, value) = row?;
            let category = match category.as_str() {
                "NOUN" => Category::Noun,
                "VERB" => Category::Verb,
                "PRONOUN" => Category::Pronoun,
                "INTERROGATIVE" => Category::Interrogative,
                _ => Category::Number,
            };
            words.entry(word).or_insert(Entry { category, key, value });
        }
        Ok(Vocabulary { words })
    }

    pub fn get(&self, word: &str) -> Option<&Entry> {
        self.words.get(word)
    }

    /// Entrada de la palabra si es de la categoría pedida.
    pub fn get_as(&self, word: &str, category: Category) -> Option<&Entry> {
        self.get(word).filter(|e| e.category == category)
    }

    /// Lee un número escrito como cifras unidas por `separator`, en la base dada.
    /// Rechaza cifras que no existen en la lengua, ceros a la izquierda y
    /// valores mayores que 9.999.
    pub fn parse_number(&self, text: &str, separator: &str, base: u32) -> Result<u32, String> {
        let parts: Vec<&str> = text.split(separator).collect();
        let mut value: u32 = 0;
        for (i, part) in parts.iter().enumerate() {
            let digit = self
                .get_as(part, Category::Number)
                .and_then(|e| e.value)
                .ok_or_else(|| format!("{part} no es una cifra de esta lengua"))?;
            if digit >= base {
                return Err(format!("{part} vale {digit}, que no cabe en base {base}"));
            }
            if i == 0 && digit == 0 && parts.len() > 1 {
                return Err("un número no empieza con cero".to_string());
            }
            value = value.saturating_mul(base).saturating_add(digit);
        }
        if value > 9_999 {
            return Err(format!("{value} pasa del máximo de 9999"));
        }
        Ok(value)
    }
}
