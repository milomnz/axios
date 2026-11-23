//! Pruebas de la base de datos y de la carga de archivos.

use axios_translator::db::{self, QuestionRecord};
use rusqlite::Connection;
use std::path::Path;

fn data(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join(name)
}

/// Base en memoria con el manual, las deducciones, el español y las señales de la semana 1.
fn initial_db() -> Connection {
    let mut conn = db::open_in_memory().unwrap();
    for f in ["manual_v0.7.txt", "deducciones_equipo.txt", "espanol_base.txt", "senales_semana1.txt"] {
        db::load_file(&mut conn, &data(f)).unwrap();
    }
    conn
}

fn count(conn: &Connection, sql: &str) -> i64 {
    conn.query_row(sql, [], |r| r.get(0)).unwrap()
}

fn current(conn: &Connection, language: &str, key: &str) -> Vec<String> {
    let mut stmt = conn
        .prepare(
            "SELECT l.word FROM lexicon l JOIN concepts c ON c.id = l.concept_id
              WHERE l.language = ?1 AND c.semantic_key = ?2 AND l.valid_to IS NULL ORDER BY l.id",
        )
        .unwrap();
    stmt.query_map([language, key], |r| r.get(0)).unwrap().map(Result::unwrap).collect()
}

#[test]
fn carga_las_15_senales_de_la_semana_1() {
    let conn = initial_db();
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM signals"), 15);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM signals WHERE voice = 'KETHARI'"), 7);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM signals WHERE voice = 'OZH'"), 7);
    // La voz desconocida del dia 6 queda registrada, pero sin transcripcion.
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM signals WHERE voice = 'UNKNOWN' AND day = 6 AND transcription IS NULL"), 1);
}

#[test]
fn el_manual_queda_en_la_base() {
    let conn = initial_db();
    assert_eq!(current(&conn, "KETHARI", "NOSOTROS"), ["anul", "anir"]);
    assert_eq!(current(&conn, "OZH", "AGUA"), ["naʔ"]);
    assert_eq!(current(&conn, "ES", "TU"), ["tu", "usted", "su"]);
    // Kethari no tiene palabras para 6, 7, 8 ni 9.
    assert!(current(&conn, "KETHARI", "NUM_6").is_empty());
    assert_eq!(current(&conn, "OZH", "NUM_6"), ["sab"]);
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM grammar_rules WHERE language = 'OZH' AND rule = 'EVIDENTIAL'"), 3);
    let base: String = conn
        .query_row("SELECT form FROM grammar_rules WHERE language = 'KETHARI' AND rule = 'NUMBER_BASE'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(base, "6");
}

#[test]
fn cargar_dos_veces_no_duplica() {
    let mut conn = initial_db();
    let before = count(&conn, "SELECT COUNT(*) FROM lexicon") + count(&conn, "SELECT COUNT(*) FROM signals");
    let report = db::load_file(&mut conn, &data("senales_semana1.txt")).unwrap();
    assert_eq!((report.signals_added, report.signals_repeated), (0, 15));
    db::load_file(&mut conn, &data("manual_v0.7.txt")).unwrap();
    let after = count(&conn, "SELECT COUNT(*) FROM lexicon") + count(&conn, "SELECT COUNT(*) FROM signals");
    assert_eq!(before, after);
}

#[test]
fn la_correccion_de_la_semana_2_versiona_teka() {
    let mut conn = initial_db();
    let before: String = conn.query_row("SELECT MAX(loaded_at) FROM sources", [], |r| r.get(0)).unwrap();

    let report = db::load_file(&mut conn, &data("senales_semana2.txt")).unwrap();
    assert_eq!((report.signals_added, report.corrections_applied), (6, 1));
    assert_eq!(report.version.as_deref(), Some("0.8"));

    // Hoy teka significa "territorio reclamado" y "tierra" ya no tiene palabra en Kethari.
    let now = db::word_report(&conn, "teka", None).unwrap();
    assert_eq!(now.concepts.len(), 1);
    assert_eq!(now.concepts[0].es, ["territorio reclamado"]);
    assert!(current(&conn, "KETHARI", "TIERRA").is_empty());
    assert_eq!(current(&conn, "OZH", "TIERRA"), ["mak"]);

    // Antes de la correccion significaba "tierra".
    let then = db::word_report(&conn, "teka", Some(&before)).unwrap();
    assert_eq!(then.concepts[0].key, "TIERRA");

    // Nada se borra: quedan las dos versiones.
    assert_eq!(now.history.len(), 2);
    assert!(now.history[0].valid_to.is_some() && now.history[1].valid_to.is_none());
}

#[test]
fn reaplicar_correcciones_o_recargar_el_manual_no_deshace_nada() {
    let mut conn = initial_db();
    db::load_file(&mut conn, &data("senales_semana2.txt")).unwrap();
    let again = db::load_file(&mut conn, &data("senales_semana2.txt")).unwrap();
    assert_eq!((again.corrections_applied, again.corrections_repeated), (0, 1));
    db::load_file(&mut conn, &data("manual_v0.7.txt")).unwrap();
    assert!(current(&conn, "KETHARI", "TIERRA").is_empty());
    assert_eq!(current(&conn, "KETHARI", "TERRITORIO_RECLAMADO"), ["teka"]);
}

#[test]
fn una_linea_mala_no_deja_nada_a_medias() {
    let mut conn = initial_db();
    let dir = std::env::temp_dir().join(format!("axios_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("malo.txt");
    std::fs::write(&file, "SENAL|20|ozh|wu|algo\nSENAL|21|marciano|x|y\n").unwrap();
    let err = db::load_file(&mut conn, &file).unwrap_err().to_string();
    assert!(err.contains("linea 2"), "{err}");
    assert_eq!(count(&conn, "SELECT COUNT(*) FROM signals WHERE day = 20"), 0);

    std::fs::write(&file, "CORRECCION|kethari|kum|agua|barco\n").unwrap();
    assert!(db::load_file(&mut conn, &file).is_err(), "kum no significa agua");
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn palabra_desconocida_no_inventa_nada() {
    let conn = initial_db();
    assert!(db::word_report(&conn, "escuchamos", None).unwrap().is_empty());
    let sal = db::word_report(&conn, "sal", None).unwrap();
    assert_eq!(sal.concepts[0].key, "ENVIAR");
    // salok (dia 1) y salok (dia 6) se reconocen como sal + -ok.
    assert_eq!(sal.signals.iter().map(|s| s.day).collect::<Vec<_>>(), [1, 6]);
    // "ir" (el) no debe confundirse con "iresh" (ellos).
    assert!(db::word_report(&conn, "ir", None).unwrap().signals.is_empty());
}

#[test]
fn ninguna_pregunta_se_pierde() {
    let conn = initial_db();
    let (id, at) = db::register_question(&conn, "kuando yegan").unwrap();
    // Aunque el proceso se cortara aqui, la pregunta ya esta guardada.
    assert_eq!(db::list_history(&conn, None).unwrap()[0].status, "RECEIVED");

    let mut record = QuestionRecord { corrected: Some("cuando llegan".into()), ..Default::default() };
    record.ozh.output = Some("ʔasur mar shavi wem".into());
    record.ozh.valid = true;
    record.kethari.error = Some("falta el sujeto".into());
    assert_eq!(db::finish_question(&conn, id, &record).unwrap(), "PARTIAL");

    let h = db::list_history(&conn, Some(&at)).unwrap();
    assert_eq!(h.len(), 1);
    assert_eq!(h[0].ozh_valid, Some(true));
    assert_eq!(h[0].kethari_valid, None);
    assert_eq!(h[0].kethari_error.as_deref(), Some("falta el sujeto"));
    assert!(db::list_history(&conn, Some("2999-01-01 00:00:00.000")).unwrap().is_empty());
}

#[test]
fn vocabulario_para_el_corrector() {
    let mut conn = initial_db();
    let words = db::spanish_words(&conn).unwrap();
    for w in ["nave", "cuando", "vendran", "el", "quince", "vivo"] {
        assert!(words.contains(&w.to_string()), "falta {w}");
    }
    assert!(db::spanish_word_exists(&conn, "tierra").unwrap());
    assert!(!db::spanish_word_exists(&conn, "yegan").unwrap());
    // Tras la correccion entra "territorio" y "tierra" sigue (en Ozh es mak).
    db::load_file(&mut conn, &data("senales_semana2.txt")).unwrap();
    assert!(db::spanish_word_exists(&conn, "territorio").unwrap());
}
