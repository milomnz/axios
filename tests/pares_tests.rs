//! Pares de pregunta y traducción esperada (rúbrica, punto 23).
//!
//! Cada par trae la frase en español, el árbol que el analizador debe
//! producir para ella y las dos traducciones, escritas a mano a partir del
//! manual. Todas deben salir tal cual y ser aceptadas por los validadores.
//! Ozh usa la partícula por defecto del equipo (`tho`) y Kethari la primera
//! forma de "nosotros" (`anul`); ver la bitácora.

use axios_translator::backends::translate_ir;
use axios_translator::db;
use axios_translator::ir::*;
use rusqlite::Connection;
use std::path::Path;

struct Par {
    espanol: &'static str,
    arbol: Ir,
    kethari: &'static str,
    ozh: &'static str,
}

fn initial_db() -> Connection {
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let mut conn = db::open_in_memory().unwrap();
    for f in ["manual_v0.7.txt", "deducciones_equipo.txt"] {
        db::load_file(&mut conn, &data.join(f)).unwrap();
    }
    conn
}

fn pron(key: &str) -> Option<Argument> {
    Some(Argument::Pronoun(key.into()))
}

/// Sustantivo con cantidad opcional; es plural si la cantidad no es uno.
fn noun(key: &str, quantity: Option<u32>) -> Option<Argument> {
    let mut np = NounPhrase::new(key);
    np.quantity = quantity.map(Quantity::Exact);
    np.plural = quantity.is_some_and(|n| n != 1);
    Some(Argument::Noun(np))
}

/// Sustantivo con poseedor.
fn owned(key: &str, owner: &str) -> Option<Argument> {
    let mut np = NounPhrase::new(key);
    np.possessor = Some(owner.into());
    Some(Argument::Noun(np))
}

fn clause(tense: Tense, negated: bool, verb: &str, subject: Option<Argument>, object: Option<Argument>) -> SemanticSentence {
    SemanticSentence { tense, negated, verb: verb.into(), subject, object }
}

fn afirma(s: SemanticSentence) -> Ir {
    Ir::Sentence(s)
}

fn pregunta(s: SemanticSentence, adjunct: Option<&str>) -> Ir {
    Ir::Question(SemanticQuestion { sentence: s, adjunct: adjunct.map(str::to_string) })
}

/// "Cuántos <sustantivo>": la cantidad es lo que se pregunta.
fn cuantos(key: &str) -> Option<Argument> {
    let mut np = NounPhrase::new(key);
    np.quantity = Some(Quantity::Unknown);
    np.plural = true;
    Some(Argument::Noun(np))
}

fn pares() -> Vec<Par> {
    use Tense::{Future as F, Past as P, Present as N};
    let wh = |k: &str| Some(Argument::Wh(k.into()));
    vec![
        Par { espanol: "Queremos paz.", arbol: afirma(clause(N, false, "QUERER", pron("NOSOTROS"), noun("PAZ", None))), kethari: "anul parn thal", ozh: "lom tho ʔenka ilu" },
        Par { espanol: "No queremos paz.", arbol: afirma(clause(N, true, "QUERER", pron("NOSOTROS"), noun("PAZ", None))), kethari: "anul parn nuthal", ozh: "ne lom tho ʔenka ilu" },
        Par { espanol: "Quisimos paz.", arbol: afirma(clause(P, false, "QUERER", pron("NOSOTROS"), noun("PAZ", None))), kethari: "anul parn thalok", ozh: "ʔoka lom tho ʔenka ilu" },
        Par { espanol: "Querremos paz.", arbol: afirma(clause(F, false, "QUERER", pron("NOSOTROS"), noun("PAZ", None))), kethari: "anul parn thalime", ozh: "venu lom tho ʔenka ilu" },
        Par { espanol: "¿Cuándo llegan?", arbol: pregunta(clause(N, false, "VENIR", pron("ELLOS"), None), Some("CUANDO")), kethari: "kom iresh vin ka", ozh: "ʔasur tho oruvi wem" },
        Par { espanol: "¿Cuándo llegarán ustedes?", arbol: pregunta(clause(F, false, "VENIR", pron("USTEDES"), None), Some("CUANDO")), kethari: "kom ulesh vinime ka", ozh: "venu ʔasur tho shavi wem" },
        Par { espanol: "¿Dónde está tu planeta?", arbol: pregunta(clause(N, false, "SER", owned("PLANETA", "TU"), None), Some("DONDE")), kethari: "dor ul vael mi ka", ozh: "ʔaha tho thüm sha wor" },
        Par { espanol: "¿Quién envió el mensaje?", arbol: pregunta(clause(P, false, "ENVIAR", wh("QUIEN"), noun("MENSAJE", None)), None), kethari: "sem ombra salok ka", ozh: "ʔoka ʔadra tho vesh wis" },
        Par { espanol: "¿Quién es usted?", arbol: pregunta(clause(N, false, "SER", pron("TU"), wh("QUIEN")), None), kethari: "sem ul mi ka", ozh: "ʔaha tho sha wis" },
        Par { espanol: "¿Qué quieren ustedes?", arbol: pregunta(clause(N, false, "QUERER", pron("USTEDES"), wh("QUE")), None), kethari: "kel ulesh thal ka", ozh: "ʔalom tho shavi wu" },
        Par { espanol: "¿Cuántas naves llegaron?", arbol: pregunta(clause(P, false, "VENIR", cuantos("NAVE"), None), None), kethari: "kai kumesh vinok ka", ozh: "ʔoka ʔasur tho gor wak" },
        Par { espanol: "¿Cuántos ciclos esperarán?", arbol: pregunta(clause(F, false, "ESPERAR", pron("ELLOS"), cuantos("CICLO")), None), kethari: "kai iresh lunesh tepime ka", ozh: "venu ʔabo tho oruvi drüm wak" },
        Par { espanol: "Llegaron quince naves.", arbol: afirma(clause(P, false, "VENIR", noun("NAVE", Some(15)), None)), kethari: "ra-sel kumesh vinok", ozh: "ʔoka sur tho ois-ulo gor" },
        Par { espanol: "Esperamos seis ciclos.", arbol: afirma(clause(N, false, "ESPERAR", pron("NOSOTROS"), noun("CICLO", Some(6)))), kethari: "anul ti-vo lunesh tep", ozh: "bo tho ʔenka sab drüm" },
        Par { espanol: "Vimos veintiséis estrellas.", arbol: afirma(clause(P, false, "VER", pron("NOSOTROS"), noun("ESTRELLA", Some(26)))), kethari: "anul mok-ra sorinesh korok", ozh: "ʔoka ʔim tho ʔenka dav-sab ral" },
        Par { espanol: "No vimos ninguna nave.", arbol: afirma(clause(P, true, "VER", pron("NOSOTROS"), noun("NAVE", None))), kethari: "anul kum nukorok", ozh: "ʔoka ne ʔim tho ʔenka gor" },
        Par { espanol: "Ellos no quieren paz.", arbol: afirma(clause(N, true, "QUERER", pron("ELLOS"), noun("PAZ", None))), kethari: "iresh parn nuthal", ozh: "ne lom tho oruvi ilu" },
        Par { espanol: "¿Es él una criatura?", arbol: pregunta(clause(N, false, "SER", pron("EL"), noun("CRIATURA", None)), None), kethari: "ir yorr mi ka", ozh: "ʔaha tho oru fen" },
        Par { espanol: "Enviamos dos mensajes.", arbol: afirma(clause(P, false, "ENVIAR", pron("NOSOTROS"), noun("MENSAJE", Some(2)))), kethari: "anul ra ombraesh salok", ozh: "ʔoka dra tho ʔenka dav vesh" },
        Par { espanol: "¿Ustedes nos escuchan?", arbol: pregunta(clause(N, false, "ESCUCHAR", pron("USTEDES"), pron("NOSOTROS")), None), kethari: "ulesh anul rhu ka", ozh: "ʔanem tho shavi ʔenka" },
        Par { espanol: "Visitaron nuestro mundo.", arbol: afirma(clause(P, false, "VISITAR", pron("ELLOS"), owned("PLANETA", "NOSOTROS"))), kethari: "iresh anul vael dunok", ozh: "ʔoka kesh tho oruvi thüm ʔenka" },
        Par { espanol: "Queremos agua.", arbol: afirma(clause(N, false, "QUERER", pron("NOSOTROS"), noun("AGUA", None))), kethari: "anul dahl thal", ozh: "lom tho ʔenka naʔ" },
        Par { espanol: "¿Quieren agua ustedes?", arbol: pregunta(clause(N, false, "QUERER", pron("USTEDES"), noun("AGUA", None)), None), kethari: "ulesh dahl thal ka", ozh: "ʔalom tho shavi naʔ" },
        Par { espanol: "Llegaron 9999 naves.", arbol: afirma(clause(P, false, "VENIR", noun("NAVE", Some(9_999)), None)), kethari: "ti-ti-mok-ti-mok-sel kumesh vinok", ozh: "ʔoka sur tho hev-hev-hev-hev gor" },
        Par { espanol: "Llegó una nave.", arbol: afirma(clause(P, false, "VENIR", noun("NAVE", Some(1)), None)), kethari: "ti kum vinok", ozh: "ʔoka sur tho ois gor" },
        Par { espanol: "No hablamos.", arbol: afirma(clause(N, true, "HABLAR", pron("NOSOTROS"), None)), kethari: "anul nular", ozh: "ne tal tho ʔenka" },
        Par { espanol: "¿Dónde están las naves?", arbol: pregunta(clause(N, false, "SER", noun("NAVE", None), None), Some("DONDE")), kethari: "dor kum mi ka", ozh: "ʔaha tho gor wor" },
        Par { espanol: "Te escucho.", arbol: afirma(clause(N, false, "ESCUCHAR", pron("YO"), pron("TU"))), kethari: "an ul rhu", ozh: "nem tho ʔen sha" },
        Par { espanol: "Queremos luz.", arbol: afirma(clause(N, false, "QUERER", pron("NOSOTROS"), noun("LUZ", None))), kethari: "anul zett thal", ozh: "lom tho ʔenka osk" },
        Par { espanol: "¿Cuándo vendrá la luz?", arbol: pregunta(clause(F, false, "VENIR", noun("LUZ", None), None), Some("CUANDO")), kethari: "kom zett vinime ka", ozh: "venu ʔasur tho osk wem" },
        Par { espanol: "Tu hogar nos espera.", arbol: afirma(clause(N, false, "ESPERAR", owned("HOGAR", "TU"), pron("NOSOTROS"))), kethari: "ul hosk anul tep", ozh: "bo tho beth sha ʔenka" },
    ]
}

#[test]
fn hay_al_menos_treinta_pares() {
    assert!(pares().len() >= 30, "hay {}", pares().len());
}

#[test]
fn cada_par_se_traduce_como_se_espera_y_pasa_los_validadores() {
    let conn = initial_db();
    for par in pares() {
        let out = translate_ir(&conn, &par.arbol);
        assert_eq!(out.kethari.output.as_deref(), Some(par.kethari), "Kethari de «{}»", par.espanol);
        assert!(out.kethari.valid, "validador de Kethari rechazo «{}»: {:?}", par.espanol, out.kethari.error);
        assert_eq!(out.ozh.output.as_deref(), Some(par.ozh), "Ozh de «{}»", par.espanol);
        assert!(out.ozh.valid, "validador de Ozh rechazo «{}»: {:?}", par.espanol, out.ozh.error);
    }
}

#[test]
fn una_lengua_sin_palabra_no_bloquea_a_la_otra_y_explica_el_motivo() {
    let mut conn = initial_db();
    let data = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    // Tras la corrección de la semana 2, "tierra" no tiene palabra en Kethari pero sí en Ozh.
    db::load_file(&mut conn, &data.join("senales_semana2.txt")).unwrap();
    let ir = afirma(clause(Tense::Present, false, "QUERER", pron("NOSOTROS"), noun("TIERRA", None)));
    let out = translate_ir(&conn, &ir);
    assert!(out.kethari.output.is_none() && !out.kethari.valid);
    assert!(out.kethari.error.as_deref().unwrap().contains("TIERRA"));
    assert_eq!(out.ozh.output.as_deref(), Some("lom tho ʔenka mak"));
    assert!(out.ozh.valid);
}

#[test]
fn la_traza_anota_generador_y_validador_de_cada_lengua() {
    let conn = initial_db();
    let ir = afirma(clause(Tense::Present, false, "QUERER", pron("NOSOTROS"), noun("PAZ", None)));
    let nombres: Vec<String> = translate_ir(&conn, &ir).trace.into_iter().map(|s| s.name).collect();
    assert_eq!(nombres, ["Generador Kethari", "Validador Kethari", "Generador Ozh", "Validador Ozh"]);
}

#[test]
fn los_validadores_recuperan_el_arbol_de_cada_par_entre_sus_lecturas() {
    use axios_translator::validation::{kethari, ozh};
    let conn = initial_db();
    for par in pares() {
        let k = kethari::validate(&conn, par.kethari).unwrap();
        assert!(k.ir == par.arbol || k.alternatives.contains(&par.arbol), "arbol de Kethari para «{}»: {:?}", par.espanol, k);
        let o = ozh::validate(&conn, par.ozh).unwrap();
        assert!(o.ir == par.arbol || o.alternatives.contains(&par.arbol), "arbol de Ozh para «{}»: {:?}", par.espanol, o);
    }
}
