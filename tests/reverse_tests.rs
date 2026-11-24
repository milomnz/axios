//! Pruebas de la traducción de regreso de Kethari y Ozh al español.
//!
//! El español de la base no lleva tildes ni artículos, y las frases ambiguas
//! devuelven una lectura por cada agrupación válida.

use axios_translator::backends::Language::{Kethari, Ozh};
use axios_translator::db;
use axios_translator::reverse::{ReverseError, to_spanish};
use rusqlite::Connection;
use std::path::Path;

fn initial_db() -> Connection {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let mut conn = db::open_in_memory().unwrap();
    for f in ["manual_v0.7.txt", "deducciones_equipo.txt", "espanol_base.txt"] {
        db::load_file(&mut conn, &data.join(f)).unwrap();
    }
    conn
}

#[test]
fn kethari_vuelve_al_espanol_con_tiempo_y_negacion() {
    let conn = initial_db();
    assert_eq!(to_spanish(&conn, Kethari, "anul parn thal").unwrap()[0], "Nosotros queremos paz.");
    assert_eq!(to_spanish(&conn, Kethari, "kom ulesh vinime ka").unwrap(), ["¿Cuando vendran ustedes?"]);
    assert_eq!(to_spanish(&conn, Kethari, "anir ulesh nukorok").unwrap(), ["Nosotros no vimos a ustedes."]);
}

#[test]
fn ozh_vuelve_al_espanol_con_particula_prefijo_y_poseedor() {
    let conn = initial_db();
    assert_eq!(to_spanish(&conn, Ozh, "ʔasur mar shavi wem").unwrap(), ["¿Cuando vienen ustedes?"]);
    assert_eq!(to_spanish(&conn, Ozh, "ʔoka dra zi ʔenka vesh").unwrap(), ["Nosotros enviamos mensaje."]);
    assert_eq!(to_spanish(&conn, Ozh, "ne lom tho oruvi ilu").unwrap(), ["Ellos no quieren paz."]);
}

#[test]
fn los_numeros_van_en_cifras_y_el_sustantivo_en_plural() {
    let conn = initial_db();
    assert_eq!(to_spanish(&conn, Kethari, "ra-sel kumesh vinok").unwrap(), ["15 naves vinieron."]);
    assert_eq!(to_spanish(&conn, Ozh, "venu sur tho ois-ulo gor").unwrap(), ["15 naves vendran."]);
    // La primera lectura es la preferida; la otra toma "anir" como poseedor.
    assert_eq!(to_spanish(&conn, Kethari, "anir mok-ra sorinesh korok").unwrap()[0], "Nosotros vimos 26 estrellas.");
}

#[test]
fn quien_y_que_ocupan_el_lugar_que_les_toca() {
    let conn = initial_db();
    assert_eq!(to_spanish(&conn, Kethari, "sem ombra salok ka").unwrap(), ["¿Quien envio mensaje?"]);
    assert_eq!(to_spanish(&conn, Ozh, "ʔanem tho shavi wis").unwrap(), ["¿Quien escuchan ustedes?"]);
}

#[test]
fn una_frase_ambigua_devuelve_las_dos_lecturas() {
    let conn = initial_db();
    // "ul vael" es "tu" + "planeta" (sujeto y objeto) o "tu planeta" (poseedor).
    let readings = to_spanish(&conn, Kethari, "ul vael vinime").unwrap();
    assert_eq!(readings.len(), 2);
    assert!(readings[0].contains("vendras"), "tu + planeta: {}", readings[0]);
    assert!(readings[1].to_lowercase().contains("planeta de tu"), "tu planeta: {}", readings[1]);
}

#[test]
fn una_frase_invalida_no_se_traduce() {
    let conn = initial_db();
    assert!(matches!(to_spanish(&conn, Kethari, "anul zorbak thal"), Err(ReverseError::Invalid(_))));
}

#[test]
fn sin_espanol_en_la_base_se_explica_que_falta() {
    let mut conn = db::open_in_memory().unwrap();
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    db::load_file(&mut conn, &data.join("manual_v0.7.txt")).unwrap();
    db::load_file(&mut conn, &data.join("deducciones_equipo.txt")).unwrap();
    // El manual trae el español de cada palabra, pero no las desinencias para conjugar.
    assert!(matches!(to_spanish(&conn, Kethari, "anul parn thal"), Err(ReverseError::MissingForm { .. })));
}
