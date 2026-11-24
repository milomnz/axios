//! Pruebas del generador de Kethari contra las señales registradas.
//!
//! Cada caso arma el árbol que produciría el parser para la frase en español y
//! compara con lo que dijo la voz Kethari. Donde el manual da dos formas
//! (`anul` y `anir`) el generador usa la primera, y el caso lo indica.

use axios_translator::backends::kethari::generate;
use axios_translator::backends::{GenError, Language};
use axios_translator::db;
use axios_translator::ir::*;
use rusqlite::Connection;
use std::path::Path;

fn initial_db() -> Connection {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let mut conn = db::open_in_memory().unwrap();
    for f in ["manual_v0.7.txt", "deducciones_equipo.txt"] {
        db::load_file(&mut conn, &data.join(f)).unwrap();
    }
    conn
}

fn pronoun(key: &str) -> Option<Argument> {
    Some(Argument::Pronoun(key.to_string()))
}

fn noun(key: &str, quantity: Option<u32>, plural: bool) -> Option<Argument> {
    let mut np = NounPhrase::new(key);
    np.quantity = quantity.map(Quantity::Exact);
    np.plural = plural;
    Some(Argument::Noun(np))
}

fn sentence(tense: Tense, negated: bool, verb: &str, subject: Option<Argument>, object: Option<Argument>) -> SemanticSentence {
    SemanticSentence { tense, negated, verb: verb.to_string(), subject, object }
}

fn statement(s: SemanticSentence) -> Ir {
    Ir::Sentence(s)
}

fn question(s: SemanticSentence, adjunct: Option<&str>) -> Ir {
    Ir::Question(SemanticQuestion { sentence: s, adjunct: adjunct.map(str::to_string) })
}

#[test]
fn dia_2_nosotros_queremos_paz() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Present, false, "QUERER", pronoun("NOSOTROS"), noun("PAZ", None, false)));
    assert_eq!(generate(&conn, &ir).unwrap(), "anul parn thal");
}

#[test]
fn dia_3_quince_naves_llegaron() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Past, false, "VENIR", noun("NAVE", Some(15), true), None));
    assert_eq!(generate(&conn, &ir).unwrap(), "ra-sel kumesh vinok");
}

#[test]
fn dia_5_cuando_vendran_ustedes() {
    let conn = initial_db();
    let ir = question(sentence(Tense::Future, false, "VENIR", pronoun("USTEDES"), None), Some("CUANDO"));
    assert_eq!(generate(&conn, &ir).unwrap(), "kom ulesh vinime ka");
}

#[test]
fn dia_6_quien_envio_el_mensaje() {
    let conn = initial_db();
    let s = sentence(Tense::Past, false, "ENVIAR", Some(Argument::Wh("QUIEN".into())), noun("MENSAJE", None, false));
    assert_eq!(generate(&conn, &question(s, None)).unwrap(), "sem ombra salok ka");
}

#[test]
fn dia_7_esperaremos_seis_ciclos() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Future, false, "ESPERAR", pronoun("NOSOTROS"), noun("CICLO", Some(6), true)));
    assert_eq!(generate(&conn, &ir).unwrap(), "anul ti-vo lunesh tepime");
}

#[test]
fn dia_9_es_el_una_criatura() {
    let conn = initial_db();
    let s = sentence(Tense::Present, false, "SER", pronoun("EL"), noun("CRIATURA", None, false));
    assert_eq!(generate(&conn, &question(s, None)).unwrap(), "ir yorr mi ka");
}

#[test]
fn dia_1_enviamos_mensajes_usa_anul_aunque_la_senal_diga_anir() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Past, false, "ENVIAR", pronoun("NOSOTROS"), noun("MENSAJE", None, true)));
    assert_eq!(generate(&conn, &ir).unwrap(), "anul ombraesh salok");
}

#[test]
fn dia_4_no_los_vimos_a_ustedes_lleva_nu_antes_del_verbo_y_ok_despues() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Past, true, "VER", pronoun("NOSOTROS"), pronoun("USTEDES")));
    assert_eq!(generate(&conn, &ir).unwrap(), "anul ulesh nukorok");
}

#[test]
fn dia_10_veintiseis_estrellas_va_en_base_6() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Past, false, "VER", pronoun("NOSOTROS"), noun("ESTRELLA", Some(26), true)));
    assert_eq!(generate(&conn, &ir).unwrap(), "anul mok-ra sorinesh korok");
}

#[test]
fn el_poseedor_va_delante_de_lo_poseido() {
    let conn = initial_db();
    let mut planeta = NounPhrase::new("PLANETA");
    planeta.possessor = Some("TU".into());
    let ir = statement(sentence(Tense::Future, false, "VENIR", Some(Argument::Noun(planeta)), None));
    assert_eq!(generate(&conn, &ir).unwrap(), "ul vael vinime");
}

#[test]
fn cuantas_naves_pone_el_interrogativo_al_inicio() {
    let conn = initial_db();
    let mut naves = NounPhrase::new("NAVE");
    naves.quantity = Some(Quantity::Unknown);
    naves.plural = true;
    let s = sentence(Tense::Past, false, "VENIR", Some(Argument::Noun(naves)), None);
    assert_eq!(generate(&conn, &question(s, None)).unwrap(), "kai kumesh vinok ka");
}

#[test]
fn una_oracion_sin_sujeto_se_rechaza() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Present, false, "QUERER", None, noun("PAZ", None, false)));
    assert!(matches!(generate(&conn, &ir), Err(GenError::MissingArgument(_))));
}

#[test]
fn una_palabra_sin_equivalente_se_explica_y_no_se_inventa() {
    let conn = initial_db();
    // El manual no trae "pez": no hay palabra que respalde la traducción.
    let ir = statement(sentence(Tense::Present, false, "VER", pronoun("YO"), noun("PEZ", None, false)));
    match generate(&conn, &ir) {
        Err(GenError::MissingWord { language, key }) => {
            assert_eq!(language, Language::Kethari);
            assert_eq!(key, "PEZ");
        }
        other => panic!("se esperaba MissingWord, salio {other:?}"),
    }
}

#[test]
fn tras_la_correccion_de_la_semana_2_tierra_no_se_traduce() {
    let mut conn = initial_db();
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    db::load_file(&mut conn, &data.join("senales_semana2.txt")).unwrap();
    let ir = statement(sentence(Tense::Present, false, "QUERER", pronoun("NOSOTROS"), noun("TIERRA", None, false)));
    assert!(matches!(generate(&conn, &ir), Err(GenError::MissingWord { .. })));
}

#[test]
fn sin_las_reglas_en_la_base_no_hay_traduccion() {
    let conn = db::open_in_memory().unwrap();
    let ir = statement(sentence(Tense::Present, false, "QUERER", pronoun("YO"), None));
    assert!(matches!(generate(&conn, &ir), Err(GenError::MissingRule { .. })));
}
