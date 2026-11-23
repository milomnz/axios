-- Esquema de la base de datos de Axios.
-- Las fechas se guardan como texto 'AAAA-MM-DD HH:MM:SS.SSS' en hora local,
-- asi se ordenan y comparan como cadenas.
-- Los intervalos de vigencia son semiabiertos: valid_from <= t < valid_to.

PRAGMA foreign_keys = ON;

-- Cada vez que se ejecuta `cargar` queda una fila: de donde salio cada dato.
CREATE TABLE IF NOT EXISTS sources (
    id          INTEGER PRIMARY KEY,
    kind        TEXT NOT NULL CHECK (kind IN ('MANUAL', 'SIGNALS', 'SPANISH', 'TEAM')),
    file_name   TEXT NOT NULL,
    version     TEXT,
    loaded_at   TEXT NOT NULL
);

-- Significados independientes de la lengua.
CREATE TABLE IF NOT EXISTS concepts (
    id           INTEGER PRIMARY KEY,
    category     TEXT NOT NULL CHECK (category IN ('NOUN', 'VERB', 'PRONOUN', 'INTERROGATIVE', 'NUMBER')),
    semantic_key TEXT NOT NULL UNIQUE,
    value        INTEGER,
    CHECK ((category = 'NUMBER') = (value IS NOT NULL))
);

-- Palabras de cada lengua con su vigencia. Una correccion del manual no borra
-- nada: cierra la fila vieja (valid_to) y abre una nueva (valid_from).
CREATE TABLE IF NOT EXISTS lexicon (
    id          INTEGER PRIMARY KEY,
    concept_id  INTEGER NOT NULL REFERENCES concepts(id),
    language    TEXT NOT NULL CHECK (language IN ('ES', 'KETHARI', 'OZH')),
    word        TEXT NOT NULL,
    note        TEXT,
    source_id   INTEGER NOT NULL REFERENCES sources(id),
    valid_from  TEXT NOT NULL,
    valid_to    TEXT,
    CHECK (valid_to IS NULL OR valid_to >= valid_from)
);
CREATE INDEX IF NOT EXISTS idx_lexicon_word ON lexicon(language, word);
CREATE UNIQUE INDEX IF NOT EXISTS ux_lexicon_current
    ON lexicon(concept_id, language, word) WHERE valid_to IS NULL;

-- Reglas gramaticales de Kethari y Ozh (afijos, particulas, orden, base numerica).
CREATE TABLE IF NOT EXISTS grammar_rules (
    id          INTEGER PRIMARY KEY,
    language    TEXT NOT NULL CHECK (language IN ('KETHARI', 'OZH')),
    rule        TEXT NOT NULL,
    position    TEXT CHECK (position IN ('PREFIX', 'SUFFIX', 'BEFORE_VERB', 'AFTER_VERB',
                                         'SENTENCE_START', 'SENTENCE_END',
                                         'BEFORE_NOUN', 'AFTER_NOUN')),
    form        TEXT NOT NULL,
    note        TEXT,
    source_id   INTEGER NOT NULL REFERENCES sources(id),
    UNIQUE (language, rule, form)
);

-- Desinencias verbales regulares del español. attaches_to dice si se pegan a la
-- raiz (habl-amos) o al infinitivo completo (llegar-an, el futuro).
CREATE TABLE IF NOT EXISTS es_verb_endings (
    id          INTEGER PRIMARY KEY,
    conjugation TEXT NOT NULL CHECK (conjugation IN ('ar', 'er', 'ir', '*')),
    tense       TEXT NOT NULL CHECK (tense IN ('PAST', 'PRESENT', 'FUTURE')),
    person      INTEGER NOT NULL CHECK (person BETWEEN 1 AND 3),
    number      TEXT NOT NULL CHECK (number IN ('SG', 'PL')),
    ending      TEXT NOT NULL,
    attaches_to TEXT NOT NULL CHECK (attaches_to IN ('STEM', 'INFINITIVE')),
    UNIQUE (conjugation, tense, person, number)
);

-- Formas verbales que no salen de las desinencias regulares.
CREATE TABLE IF NOT EXISTS es_irregular_forms (
    form        TEXT NOT NULL,
    infinitive  TEXT NOT NULL,
    tense       TEXT NOT NULL CHECK (tense IN ('PAST', 'PRESENT', 'FUTURE')),
    person      INTEGER NOT NULL CHECK (person BETWEEN 1 AND 3),
    number      TEXT NOT NULL CHECK (number IN ('SG', 'PL')),
    PRIMARY KEY (form, infinitive)
);

-- Palabras gramaticales del español que no tienen traduccion propia.
CREATE TABLE IF NOT EXISTS es_function_words (
    word        TEXT PRIMARY KEY,
    kind        TEXT NOT NULL CHECK (kind IN ('ARTICLE', 'PREPOSITION', 'NEGATION',
                                              'CONJUNCTION', 'AFFIRMATION', 'AUXILIARY'))
);

-- Señales recibidas. transcription es NULL cuando no hubo transcripcion
-- (la voz desconocida del dia 6).
CREATE TABLE IF NOT EXISTS signals (
    id             INTEGER PRIMARY KEY,
    day            INTEGER NOT NULL CHECK (day > 0),
    voice          TEXT NOT NULL CHECK (voice IN ('KETHARI', 'OZH', 'UNKNOWN')),
    transcription  TEXT,
    decoder_output TEXT,
    source_id      INTEGER NOT NULL REFERENCES sources(id)
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_signal
    ON signals(day, voice, IFNULL(transcription, ''));

-- Correcciones del manual tal como llegaron, enlazadas a las filas del lexico
-- que cerraron y abrieron.
CREATE TABLE IF NOT EXISTS corrections (
    id              INTEGER PRIMARY KEY,
    language        TEXT NOT NULL CHECK (language IN ('ES', 'KETHARI', 'OZH')),
    word            TEXT NOT NULL,
    old_meaning     TEXT NOT NULL,
    new_meaning     TEXT NOT NULL,
    old_lexicon_id  INTEGER NOT NULL REFERENCES lexicon(id),
    new_lexicon_id  INTEGER NOT NULL REFERENCES lexicon(id),
    source_id       INTEGER NOT NULL REFERENCES sources(id),
    applied_at      TEXT NOT NULL
);

-- Cada pregunta recibida. La fila se crea antes de traducir (status RECEIVED)
-- para que ninguna pregunta se pierda aunque el proceso falle a mitad.
CREATE TABLE IF NOT EXISTS translations (
    id               INTEGER PRIMARY KEY,
    created_at       TEXT NOT NULL,
    raw_input        TEXT NOT NULL,
    normalized_input TEXT,
    corrected_input  TEXT,
    kethari_output   TEXT,
    kethari_valid    INTEGER CHECK (kethari_valid IN (0, 1)),
    kethari_error    TEXT,
    ozh_output       TEXT,
    ozh_valid        INTEGER CHECK (ozh_valid IN (0, 1)),
    ozh_error        TEXT,
    status           TEXT NOT NULL CHECK (status IN ('RECEIVED', 'TRANSLATED', 'PARTIAL', 'REJECTED')),
    error_reason     TEXT
);
CREATE INDEX IF NOT EXISTS idx_translations_date ON translations(created_at);
