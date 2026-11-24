//! Pruebas de los validadores de Kethari y Ozh.

use axios_translator::backends::{kethari as kethari_gen, ozh as ozh_gen};
use axios_translator::db;
use axios_translator::ir::*;
use axios_translator::validation::{ValidationError, TokenKind, kethari, ozh};
use rusqlite::Connection;
use std::path::Path;

fn data(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join(name)
}

/// Base con el manual, las deducciones y las señales de las dos semanas.
fn initial_db() -> Connection {
    let mut conn = db::open_in_memory().unwrap();
    for f in ["manual_v0.7.txt", "deducciones_equipo.txt", "senales_semana1.txt", "senales_semana2.txt"] {
        db::load_file(&mut conn, &data(f)).unwrap();
    }
    conn
}

/// Posición y motivo del rechazo, para comprobar que explica dónde falla.
fn rejection(result: Result<impl std::fmt::Debug, ValidationError>) -> (usize, String) {
    match result {
        Err(ValidationError::Invalid { position, message }) => (position, message),
        other => panic!("se esperaba un rechazo, salio {other:?}"),
    }
}

/// Transcripciones de un archivo de señales para una voz.
fn transcriptions(file: &str, voice: &str) -> Vec<String> {
    std::fs::read_to_string(data(file))
        .unwrap()
        .lines()
        .filter(|l| l.starts_with("SENAL|"))
        .map(|l| l.split('|').collect::<Vec<_>>())
        .filter(|f| f[2] == voice && !f[3].is_empty())
        .map(|f| f[3].to_string())
        .collect()
}

#[test]
fn acepta_todas_las_senales_registradas_de_kethari() {
    let conn = initial_db();
    for file in ["senales_semana1.txt", "senales_semana2.txt"] {
        for text in transcriptions(file, "kethari") {
            kethari::validate(&conn, &text).unwrap_or_else(|e| panic!("{text}: {e}"));
        }
    }
}

#[test]
fn acepta_todas_las_senales_registradas_de_ozh() {
    let conn = initial_db();
    for file in ["senales_semana1.txt", "senales_semana2.txt"] {
        for text in transcriptions(file, "ozh") {
            ozh::validate(&conn, &text).unwrap_or_else(|e| panic!("{text}: {e}"));
        }
    }
}

#[test]
fn lee_una_pregunta_de_kethari() {
    let conn = initial_db();
    let reading = kethari::validate(&conn, "kom ulesh vinime ka").unwrap();
    assert!(reading.is_question);
    assert_eq!(reading.tokens.len(), 4);
    assert_eq!(reading.tokens[2].kind, TokenKind::KethariVerb { key: "VENIR".into(), negated: false, tense: Tense::Future });
    assert_eq!(reading.tokens[2].start, 10);
}

#[test]
fn separa_prefijo_y_sufijo_del_verbo_negado_en_pasado() {
    let conn = initial_db();
    let reading = kethari::validate(&conn, "anir ulesh nukorok").unwrap();
    assert_eq!(reading.tokens[2].kind, TokenKind::KethariVerb { key: "VER".into(), negated: true, tense: Tense::Past });
}

#[test]
fn enka_en_ozh_no_se_lee_como_en_mas_ka() {
    let conn = initial_db();
    let reading = ozh::validate(&conn, "ʔoka dra zi ʔenka vesh").unwrap();
    assert_eq!(reading.tokens[3].kind, TokenKind::Pronoun { key: "NOSOTROS".into() });
    // En Kethari ʔenka no existe: no se parte para encontrar un `ka`.
    let (position, _) = rejection(kethari::validate(&conn, "ʔenka parn thal"));
    assert_eq!(position, 0);
}

#[test]
fn rechaza_una_palabra_inventada_y_dice_donde() {
    let conn = initial_db();
    let (position, message) = rejection(kethari::validate(&conn, "anul zorbak thal"));
    assert_eq!(position, 5);
    assert!(message.contains("zorbak"));
}

#[test]
fn kethari_rechaza_el_verbo_fuera_de_su_lugar() {
    let conn = initial_db();
    // El verbo debe cerrar la frase: se señala la última palabra, que no lo es.
    let (position, _) = rejection(kethari::validate(&conn, "thal anul parn"));
    assert_eq!(position, 10);
}

#[test]
fn kethari_rechaza_la_pregunta_sin_ka_y_el_ka_sin_pregunta() {
    let conn = initial_db();
    rejection(kethari::validate(&conn, "kom ulesh vinime"));
    // Con ka y sin interrogativo es una pregunta de sí o no, que sí es válida.
    assert!(kethari::validate(&conn, "ir yorr mi ka").unwrap().is_question);
    // El ka en medio de la frase no es válido.
    rejection(kethari::validate(&conn, "anul ka parn thal"));
}

#[test]
fn kethari_exige_sujeto_salvo_que_lo_ocupe_el_interrogativo() {
    let conn = initial_db();
    rejection(kethari::validate(&conn, "thal"));
    assert!(kethari::validate(&conn, "sem ombra salok ka").is_ok());
}

#[test]
fn kethari_exige_plural_con_numeros_mayores_que_uno() {
    let conn = initial_db();
    rejection(kethari::validate(&conn, "ra-sel kum vinok"));
    rejection(kethari::validate(&conn, "ti kumesh vinok"));
    assert!(kethari::validate(&conn, "ti kum vinok").is_ok());
}

#[test]
fn kethari_rechaza_numeros_mal_formados() {
    let conn = initial_db();
    // Cero a la izquierda, y 6 y 7 no existen como cifras en Kethari.
    rejection(kethari::validate(&conn, "vo-ti kumesh vinok"));
    rejection(kethari::validate(&conn, "sab kumesh vinok"));
}

#[test]
fn ozh_exige_la_particula_despues_del_verbo() {
    let conn = initial_db();
    let (position, message) = rejection(ozh::validate(&conn, "lom oruvi ilu"));
    assert_eq!(position, 4);
    assert!(message.contains("partícula"));
}

#[test]
fn ozh_rechaza_la_negacion_despues_del_verbo() {
    let conn = initial_db();
    rejection(ozh::validate(&conn, "lom ne tho oruvi ilu"));
}

#[test]
fn ozh_pide_el_prefijo_de_pregunta_si_hay_interrogativo() {
    let conn = initial_db();
    rejection(ozh::validate(&conn, "sur mar shavi wem"));
    let reading = ozh::validate(&conn, "ʔasur mar shavi wem").unwrap();
    assert!(reading.is_question);
}

#[test]
fn ozh_acepta_el_cero_y_rechaza_ceros_a_la_izquierda() {
    let conn = initial_db();
    assert!(ozh::validate(&conn, "sur tho zu gor").is_ok());
    rejection(ozh::validate(&conn, "sur tho zu-ois gor"));
}

#[test]
fn ozh_rechaza_una_palabra_de_kethari() {
    let conn = initial_db();
    let (position, _) = rejection(ozh::validate(&conn, "dahl thal ulesh"));
    assert_eq!(position, 0);
}

#[test]
fn lee_lo_que_generan_los_generadores() {
    let conn = initial_db();
    let pron = |k: &str| Some(Argument::Pronoun(k.to_string()));
    let mut naves = NounPhrase::new("NAVE");
    naves.quantity = Some(Quantity::Exact(26));
    let sentences = [
        SemanticSentence { tense: Tense::Past, negated: true, verb: "VER".into(), subject: pron("NOSOTROS"), object: Some(Argument::Noun(naves)) },
        SemanticSentence { tense: Tense::Future, negated: false, verb: "VENIR".into(), subject: pron("USTEDES"), object: None },
        SemanticSentence { tense: Tense::Present, negated: false, verb: "ENVIAR".into(), subject: Some(Argument::Wh("QUIEN".into())), object: Some(Argument::Noun(NounPhrase::new("MENSAJE"))) },
    ];
    for s in sentences {
        for ir in [Ir::Sentence(s.clone()), Ir::Question(SemanticQuestion { sentence: s.clone(), adjunct: None })] {
            let k = kethari_gen::generate(&conn, &ir).unwrap();
            kethari::validate(&conn, &k).unwrap_or_else(|e| panic!("{k}: {e}"));
            let o = ozh_gen::generate(&conn, &ir).unwrap();
            ozh::validate(&conn, &o).unwrap_or_else(|e| panic!("{o}: {e}"));
        }
    }
}
