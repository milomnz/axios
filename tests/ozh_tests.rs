//! Pruebas del generador de Ozh contra las señales registradas.
//!
//! Cada caso arma el árbol que produciría el parser y lo compara con lo que
//! dijo la voz Ozh. La partícula evidencial se fija con `generate_with` para
//! reproducir la de cada señal; `generate` usa la del equipo.

use axios_translator::backends::ozh::{generate, generate_with};
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

fn noun(key: &str, quantity: Option<u32>) -> Option<Argument> {
    let mut np = NounPhrase::new(key);
    np.quantity = quantity.map(Quantity::Exact);
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
fn dia_1_antes_enviamos_un_mensaje() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Past, false, "ENVIAR", pronoun("NOSOTROS"), noun("MENSAJE", None)));
    assert_eq!(generate_with(&conn, &ir, "zi").unwrap(), "ʔoka dra zi ʔenka vesh");
}

#[test]
fn dia_2_ellos_quieren_paz() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Present, false, "QUERER", pronoun("ELLOS"), noun("PAZ", None)));
    assert_eq!(generate_with(&conn, &ir, "tho").unwrap(), "lom tho oruvi ilu");
}

#[test]
fn dia_3_llegaran_quince_naves() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Future, false, "VENIR", noun("NAVE", Some(15)), None));
    assert_eq!(generate_with(&conn, &ir, "tho").unwrap(), "venu sur tho ois-ulo gor");
}

#[test]
fn dia_5_cuando_vienen_ustedes_lleva_prefijo_y_el_interrogativo_al_final() {
    let conn = initial_db();
    let ir = question(sentence(Tense::Present, false, "VENIR", pronoun("USTEDES"), None), Some("CUANDO"));
    assert_eq!(generate_with(&conn, &ir, "mar").unwrap(), "ʔasur mar shavi wem");
}

#[test]
fn dia_6_ustedes_no_hablan_lleva_ne_antes_del_verbo() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Present, true, "HABLAR", pronoun("USTEDES"), None));
    assert_eq!(generate_with(&conn, &ir, "mar").unwrap(), "ne tal mar shavi");
}

#[test]
fn dia_7_yo_espero_seis_ciclos_no_marca_plural() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Present, false, "ESPERAR", pronoun("YO"), noun("CICLO", Some(6))));
    assert_eq!(generate_with(&conn, &ir, "mar").unwrap(), "bo mar ʔen sab drüm");
}

#[test]
fn dia_8_visitaron_nuestro_mundo_pone_el_poseedor_detras() {
    let conn = initial_db();
    let mut mundo = NounPhrase::new("PLANETA");
    mundo.possessor = Some("NOSOTROS".into());
    let ir = statement(sentence(Tense::Past, false, "VISITAR", pronoun("ELLOS"), Some(Argument::Noun(mundo))));
    assert_eq!(generate_with(&conn, &ir, "zi").unwrap(), "ʔoka kesh zi oruvi thüm ʔenka");
}

#[test]
fn dia_9_a_quien_escuchan_ustedes_mueve_el_interrogativo_al_final() {
    let conn = initial_db();
    let s = sentence(Tense::Present, false, "ESCUCHAR", pronoun("USTEDES"), Some(Argument::Wh("QUIEN".into())));
    assert_eq!(generate_with(&conn, &question(s, None), "tho").unwrap(), "ʔanem tho shavi wis");
}

#[test]
fn dia_10_ellos_no_quieren_paz() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Present, true, "QUERER", pronoun("ELLOS"), noun("PAZ", None)));
    assert_eq!(generate_with(&conn, &ir, "tho").unwrap(), "ne lom tho oruvi ilu");
}

#[test]
fn dia_4_en_pasado_el_manual_pide_oka_aunque_la_senal_no_lo_traiga() {
    let conn = initial_db();
    // La señal dice "ne ʔim zi ʔenka gor" sin ʔoka; se sigue la regla 3.4.3 del manual.
    let ir = statement(sentence(Tense::Past, true, "VER", pronoun("NOSOTROS"), noun("NAVE", None)));
    assert_eq!(generate_with(&conn, &ir, "zi").unwrap(), "ʔoka ne ʔim zi ʔenka gor");
}

#[test]
fn la_particula_por_defecto_sale_de_la_base() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Present, false, "QUERER", pronoun("NOSOTROS"), noun("PAZ", None)));
    assert_eq!(generate(&conn, &ir).unwrap(), "lom tho ʔenka ilu");
}

#[test]
fn una_particula_fuera_del_manual_se_rechaza() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Present, false, "QUERER", pronoun("YO"), None));
    assert!(matches!(generate_with(&conn, &ir, "xyz"), Err(GenError::UnsupportedRule { .. })));
}

#[test]
fn cuantas_naves_pone_cuantos_al_final_sin_numero() {
    let conn = initial_db();
    let mut naves = NounPhrase::new("NAVE");
    naves.quantity = Some(Quantity::Unknown);
    let s = sentence(Tense::Past, false, "VENIR", Some(Argument::Noun(naves)), None);
    assert_eq!(generate_with(&conn, &question(s, None), "tho").unwrap(), "ʔoka ʔasur tho gor wak");
}

#[test]
fn una_palabra_sin_equivalente_en_ozh_se_explica() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Present, false, "VER", pronoun("YO"), noun("PEZ", None)));
    match generate(&conn, &ir) {
        Err(GenError::MissingWord { language, key }) => {
            assert_eq!(language, Language::Ozh);
            assert_eq!(key, "PEZ");
        }
        other => panic!("se esperaba MissingWord, salio {other:?}"),
    }
}

#[test]
fn una_oracion_sin_sujeto_se_rechaza() {
    let conn = initial_db();
    let ir = statement(sentence(Tense::Present, false, "QUERER", None, noun("PAZ", None)));
    assert!(matches!(generate(&conn, &ir), Err(GenError::MissingArgument(_))));
}

#[test]
fn sin_las_reglas_en_la_base_no_hay_traduccion() {
    let conn = db::open_in_memory().unwrap();
    let ir = statement(sentence(Tense::Present, false, "QUERER", pronoun("YO"), None));
    assert!(matches!(generate(&conn, &ir), Err(GenError::MissingRule { .. })));
}
