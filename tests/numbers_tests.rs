//! Pruebas de los números en Kethari y Ozh contra la base con el manual cargado.

use axios_translator::backends::Language::{Kethari, Ozh};
use axios_translator::backends::numbers::{NumberError, number_words};
use axios_translator::db;
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

#[test]
fn kethari_usa_base_6_como_en_las_senales() {
    let conn = initial_db();
    // Día 3: quince naves; día 7: seis ciclos; día 10: veintiséis estrellas.
    assert_eq!(number_words(&conn, Kethari, 15).unwrap(), "ra-sel");
    assert_eq!(number_words(&conn, Kethari, 6).unwrap(), "ti-vo");
    assert_eq!(number_words(&conn, Kethari, 26).unwrap(), "mok-ra");
}

#[test]
fn ozh_usa_base_10_como_en_las_senales() {
    let conn = initial_db();
    assert_eq!(number_words(&conn, Ozh, 15).unwrap(), "ois-ulo");
    assert_eq!(number_words(&conn, Ozh, 6).unwrap(), "sab");
}

#[test]
fn el_cero_y_el_maximo_se_traducen() {
    let conn = initial_db();
    assert_eq!(number_words(&conn, Kethari, 0).unwrap(), "vo");
    assert_eq!(number_words(&conn, Ozh, 0).unwrap(), "zu");
    assert_eq!(number_words(&conn, Ozh, 9_999).unwrap(), "hev-hev-hev-hev");
    assert_eq!(number_words(&conn, Kethari, 9_999).unwrap(), "ti-ti-mok-ti-mok-sel");
}

#[test]
fn pasar_del_maximo_se_rechaza() {
    let conn = initial_db();
    assert!(matches!(number_words(&conn, Ozh, 10_000), Err(NumberError::OutOfRange(10_000))));
}

#[test]
fn sin_reglas_en_la_base_no_se_inventa_nada() {
    let conn = db::open_in_memory().unwrap();
    assert!(matches!(number_words(&conn, Kethari, 5), Err(NumberError::MissingRule { .. })));
}
