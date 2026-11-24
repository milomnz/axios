//! Representación intermedia (IR): el árbol que describe una pregunta o una
//! oración sin depender de ninguna lengua.
//!
//! Lo produce el analizador sintáctico del español (Integrante 3) y lo leen los
//! generadores de Kethari y Ozh (Integrante 4). Es el contrato entre las dos
//! partes. Ninguna palabra de una lengua vive aquí: todo se nombra con la
//! `semantic_key` de la tabla `concepts` ("VENIR", "NAVE", "NOSOTROS"), y cada
//! generador la busca en el léxico.
//!
//! Estado: primera versión, a la espera de que el analizador del español la
//! complete. Lo que falta está en `docs/arbol_ir.md`.

/// Tiempo verbal. "Van a llegar" ya llega aquí como `Future`: la perífrasis
/// `ir + a + infinitivo` la resuelve la morfología, no los generadores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tense {
    Past,
    Present,
    Future,
}

/// Cantidad de un sintagma nominal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    /// Un número, de 0 a 9.999.
    Exact(u32),
    /// "Cuántos": la cantidad es lo que se pregunta.
    Unknown,
}

/// Un sustantivo con lo que lo acompaña.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NounPhrase {
    /// `semantic_key` del sustantivo, por ejemplo "NAVE".
    pub noun: String,
    pub quantity: Option<Quantity>,
    /// Plural gramatical del español ("naves"). Ver `is_plural`.
    pub plural: bool,
    /// `semantic_key` del pronombre poseedor ("NOSOTROS" en "nuestro mundo").
    pub possessor: Option<String>,
}

impl NounPhrase {
    pub fn new(noun: &str) -> Self {
        NounPhrase { noun: noun.to_string(), quantity: None, plural: false, possessor: None }
    }

    /// Plural para los generadores: el plural gramatical o una cantidad mayor
    /// que uno. Así "quince naves" y "naves" se tratan igual.
    pub fn is_plural(&self) -> bool {
        self.plural || matches!(self.quantity, Some(Quantity::Exact(n)) if n > 1)
    }
}

/// Lo que puede ocupar el sujeto o el objeto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Argument {
    /// `semantic_key` de un pronombre ("YO", "USTEDES"...).
    Pronoun(String),
    Noun(NounPhrase),
    /// Palabra interrogativa que ocupa el lugar del argumento: "quién" en
    /// "quién envió el mensaje", "qué" en "qué quieren". La clave es la
    /// `semantic_key` ("QUIEN", "QUE").
    Wh(String),
}

/// Una oración simple: verbo con sujeto y objeto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticSentence {
    pub tense: Tense,
    pub negated: bool,
    /// `semantic_key` del verbo, por ejemplo "VENIR".
    pub verb: String,
    /// El analizador debe completarlo cuando el español lo omite
    /// ("Cuando llegan?" -> ELLOS), porque las dos lenguas exigen sujeto.
    pub subject: Option<Argument>,
    /// Objeto directo o complemento ("Es el una criatura" -> CRIATURA).
    pub object: Option<Argument>,
}

/// Una pregunta: una oración más lo que se pregunta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticQuestion {
    pub sentence: SemanticSentence,
    /// Interrogativo que no es sujeto ni objeto: "cuándo" y "dónde".
    /// Si tampoco hay un `Argument::Wh` ni una cantidad `Unknown`,
    /// la pregunta es de sí o no.
    pub adjunct: Option<String>,
}

impl SemanticQuestion {
    /// El interrogativo que cada lengua debe mover a su posición: al inicio en
    /// Kethari, al final en Ozh. `None` en las preguntas de sí o no.
    pub fn wh_word(&self) -> Option<&str> {
        if let Some(a) = &self.adjunct {
            return Some(a);
        }
        [&self.sentence.subject, &self.sentence.object].into_iter().flatten().find_map(|arg| match arg {
            Argument::Wh(key) => Some(key.as_str()),
            Argument::Noun(np) if np.quantity == Some(Quantity::Unknown) => Some("CUANTOS"),
            _ => None,
        })
    }

    pub fn is_yes_no(&self) -> bool {
        self.wh_word().is_none()
    }
}

/// Lo que el analizador entrega a los generadores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ir {
    Sentence(SemanticSentence),
    Question(SemanticQuestion),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pronoun(key: &str) -> Option<Argument> {
        Some(Argument::Pronoun(key.to_string()))
    }

    #[test]
    fn la_cantidad_mayor_que_uno_hace_plural() {
        let mut naves = NounPhrase::new("NAVE");
        assert!(!naves.is_plural());
        naves.quantity = Some(Quantity::Exact(15));
        assert!(naves.is_plural());
        naves.quantity = Some(Quantity::Exact(1));
        assert!(!naves.is_plural());
    }

    #[test]
    fn cuando_llegan_es_pregunta_con_adjunto() {
        // "Cuando llegan?" con el sujeto omitido, ya completado como ELLOS.
        let q = SemanticQuestion {
            sentence: SemanticSentence {
                tense: Tense::Present,
                negated: false,
                verb: "VENIR".into(),
                subject: pronoun("ELLOS"),
                object: None,
            },
            adjunct: Some("CUANDO".into()),
        };
        assert_eq!(q.wh_word(), Some("CUANDO"));
        assert!(!q.is_yes_no());
    }

    #[test]
    fn quien_envio_el_mensaje_lleva_el_interrogativo_como_sujeto() {
        let q = SemanticQuestion {
            sentence: SemanticSentence {
                tense: Tense::Past,
                negated: false,
                verb: "ENVIAR".into(),
                subject: Some(Argument::Wh("QUIEN".into())),
                object: Some(Argument::Noun(NounPhrase::new("MENSAJE"))),
            },
            adjunct: None,
        };
        assert_eq!(q.wh_word(), Some("QUIEN"));
    }

    #[test]
    fn es_el_una_criatura_es_de_si_o_no() {
        let q = SemanticQuestion {
            sentence: SemanticSentence {
                tense: Tense::Present,
                negated: false,
                verb: "SER".into(),
                subject: pronoun("EL"),
                object: Some(Argument::Noun(NounPhrase::new("CRIATURA"))),
            },
            adjunct: None,
        };
        assert!(q.is_yes_no());
    }

    #[test]
    fn cuantas_naves_es_pregunta_por_la_cantidad() {
        let mut naves = NounPhrase::new("NAVE");
        naves.quantity = Some(Quantity::Unknown);
        let q = SemanticQuestion {
            sentence: SemanticSentence {
                tense: Tense::Past,
                negated: false,
                verb: "VENIR".into(),
                subject: Some(Argument::Noun(naves)),
                object: None,
            },
            adjunct: None,
        };
        assert!(!q.is_yes_no());
    }
}
