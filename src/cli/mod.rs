//! Interfaz de linea de ordenes: traducir, cargar, historial y palabra.

use crate::db::{self, HistoryEntry, WordReport};
use crate::error::Result;
use std::path::{Path, PathBuf};

const USAGE: &str = "Uso:
  axios traducir [--traza] \"<pregunta>\"   Traduce a Kethari y Ozh y lo registra todo
  axios cargar <archivo>                  Incorpora un archivo de señales, correcciones o lexico
  axios historial [desde <fecha>]         Lista lo traducido y su resultado
  axios palabra <termino> [en <fecha>]    Lo que se sabe de una palabra (hoy o en esa fecha)

Fechas: AAAA-MM-DD o AAAA-MM-DD HH:MM[:SS[.mmm]]
Base de datos: axios.db en la carpeta actual, o la ruta de la variable AXIOS_DB";

#[derive(Debug, PartialEq)]
pub enum Command {
    Translate { text: String, trace: bool },
    Load { path: PathBuf },
    History { since: Option<String> },
    Word { term: String, at: Option<String> },
}

pub fn parse(args: &[String]) -> std::result::Result<Command, String> {
    let (order, rest) = args.split_first().ok_or("falta la orden")?;
    match order.as_str() {
        "traducir" => {
            let trace = rest.first().is_some_and(|a| a == "--traza");
            let text = rest[trace as usize..].join(" ");
            if text.trim().is_empty() {
                return Err("falta la pregunta".into());
            }
            Ok(Command::Translate { text, trace })
        }
        "cargar" => match rest {
            [path] => Ok(Command::Load { path: PathBuf::from(path) }),
            _ => Err("cargar recibe exactamente un archivo".into()),
        },
        "historial" => match rest {
            [] => Ok(Command::History { since: None }),
            [desde, date @ ..] if desde == "desde" && !date.is_empty() => {
                let since = db::complete_date(&date.join(" "), false).ok_or("fecha no valida")?;
                Ok(Command::History { since: Some(since) })
            }
            _ => Err("se esperaba: historial [desde <fecha>]".into()),
        },
        "palabra" => {
            let (term, at) = match rest.iter().position(|a| a == "en") {
                Some(i) if i > 0 => {
                    let at = db::complete_date(&rest[i + 1..].join(" "), true).ok_or("fecha no valida")?;
                    (rest[..i].join(" "), Some(at))
                }
                _ => (rest.join(" "), None),
            };
            if term.trim().is_empty() {
                return Err("falta el termino".into());
            }
            Ok(Command::Word { term, at })
        }
        other => Err(format!("orden desconocida «{other}»")),
    }
}

/// Ejecuta la orden y devuelve el codigo de salida del proceso.
pub fn run(args: &[String]) -> i32 {
    let command = match parse(args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{e}\n\n{USAGE}");
            return 2;
        }
    };
    let path = std::env::var_os("AXIOS_DB").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("axios.db"));
    match execute(&path, command) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

fn execute(db_path: &Path, command: Command) -> Result<()> {
    let mut conn = db::open(db_path)?;
    match command {
        Command::Translate { text, trace } => translate(&conn, &text, trace),
        Command::Load { path } => {
            let report = db::load_file(&mut conn, &path)?;
            print!("Cargado {}\n{report}", path.display());
            Ok(())
        }
        Command::History { since } => {
            let entries = db::list_history(&conn, since.as_deref())?;
            if entries.is_empty() {
                println!("No hay preguntas registradas{}.", since.map(|s| format!(" desde {s}")).unwrap_or_default());
            }
            for e in &entries {
                print_entry(e);
            }
            Ok(())
        }
        Command::Word { term, at } => {
            print_word(&db::word_report(&conn, &term, at.as_deref())?);
            Ok(())
        }
    }
}

fn translate(conn: &rusqlite::Connection, text: &str, trace: bool) -> Result<()> {
    // Primero se registra: aunque algo falle despues, la pregunta no se pierde.
    let (id, at) = db::register_question(conn, text)?;
    let out = crate::translate(conn, text);
    let status = db::finish_question(conn, id, &out.record)?;

    println!("Pregunta #{id} registrada el {at}");
    if trace {
        for stage in &out.trace {
            println!("  [{}] {}", stage.name, stage.output);
        }
    }
    let r = &out.record;
    if let Some(c) = r.corrected.as_ref().filter(|c| Some(*c) != r.normalized.as_ref()) {
        println!("Entendido como: {c}");
    }
    for (name, lang) in [("Kethari", &r.kethari), ("Ozh", &r.ozh)] {
        match (&lang.output, lang.valid) {
            (Some(o), true) => println!("{name:<8} {o}"),
            (Some(o), false) => println!("{name:<8} NO SE TRANSMITE «{o}»: {}", lang.error.as_deref().unwrap_or("el validador la rechazo")),
            (None, _) => println!("{name:<8} sin traduccion: {}", lang.error.as_deref().or(r.error_reason.as_deref()).unwrap_or("sin motivo registrado")),
        }
    }
    println!("Estado: {}", status_es(status));
    Ok(())
}

fn status_es(status: &str) -> &str {
    match status {
        "TRANSLATED" => "traducida a las dos lenguas",
        "PARTIAL" => "traducida solo a una lengua",
        "REJECTED" => "sin traduccion",
        _ => "recibida, sin terminar",
    }
}

fn print_entry(e: &HistoryEntry) {
    println!("#{} {}  {}", e.id, e.created_at, status_es(&e.status));
    println!("    pregunta:   {}", e.raw_input);
    if let Some(c) = &e.corrected_input {
        println!("    corregida:  {c}");
    }
    for (name, out, valid, err) in [
        ("kethari:", &e.kethari_output, e.kethari_valid, &e.kethari_error),
        ("ozh:", &e.ozh_output, e.ozh_valid, &e.ozh_error),
    ] {
        match (out, valid) {
            (Some(o), Some(true)) => println!("    {name:<11} {o}  [valida]"),
            (Some(o), _) => println!("    {name:<11} {o}  [rechazada: {}]", err.as_deref().unwrap_or("-")),
            (None, _) => {
                if let Some(err) = err {
                    println!("    {name:<11} - ({err})");
                }
            }
        }
    }
    if let Some(r) = &e.error_reason {
        println!("    motivo:     {r}");
    }
}

fn print_word(w: &WordReport) {
    let when = w.at.as_deref().map(|t| format!(" el {t}")).unwrap_or_default();
    if w.is_empty() {
        println!("No hay nada registrado sobre «{}» en ninguna de las tres lenguas.", w.term);
        return;
    }
    println!("«{}»{when}", w.term);
    if w.concepts.is_empty() && !w.history.is_empty() {
        println!("  No tenia ningun significado vigente{}.", if when.is_empty() { " ahora".into() } else { when.clone() });
    }
    for c in &w.concepts {
        let value = c.value.map(|v| format!(" = {v}")).unwrap_or_default();
        println!("  {} ({}{value})", c.key, c.category);
        for (lang, words) in [("español", &c.es), ("kethari", &c.kethari), ("ozh", &c.ozh)] {
            let shown = if words.is_empty() { "(sin palabra conocida)".to_string() } else { words.join(", ") };
            println!("    {lang:<8} {shown}");
        }
        for n in &c.notes {
            println!("    nota: {n}");
        }
    }
    let changed = w.history.iter().any(|v| v.valid_to.is_some());
    if changed {
        println!("  Versiones:");
        for v in &w.history {
            let to = v.valid_to.as_deref().unwrap_or("hoy");
            println!("    {:<8} «{}» = {}  [{} → {}]  {}", v.language, v.word, v.meaning, v.valid_from, to, v.source);
        }
    }
    for r in &w.rules {
        println!("  {r}");
    }
    for g in &w.spanish_grammar {
        println!("  español: {g}");
    }
    if !w.signals.is_empty() {
        println!("  Aparece en señales:");
        for s in &w.signals {
            println!("    dia {} {:<7} {}  ({})  → {}", s.day, s.voice, s.transcription, s.form, s.decoder_output);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn lee_ordenes() {
        assert_eq!(
            parse(&args(&["traducir", "--traza", "cuando", "llegan"])).unwrap(),
            Command::Translate { text: "cuando llegan".into(), trace: true }
        );
        assert_eq!(
            parse(&args(&["palabra", "teka", "en", "2026-11-20"])).unwrap(),
            Command::Word { term: "teka".into(), at: Some("2026-11-20 23:59:59.999".into()) }
        );
        assert_eq!(
            parse(&args(&["historial", "desde", "2026-11-20"])).unwrap(),
            Command::History { since: Some("2026-11-20 00:00:00.000".into()) }
        );
        assert!(parse(&args(&["traducir"])).is_err());
        assert!(parse(&args(&["historial", "ayer"])).is_err());
        assert!(parse(&args(&["borrar"])).is_err());
    }
}
