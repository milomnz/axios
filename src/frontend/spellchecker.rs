/// Contrato que implementará la BD SQLite del Integrante 1.
/// Mientras tanto, tú lo implementas en `tests/frontend_int2_tests.rs` con un Mock.
pub trait SpanishLexicon {
    /// Verifica si la palabra existe tal cual en el vocabulario vigente.
    fn exists(&self, word: &str) -> bool;
    /// Devuelve el listado de palabras válidas en español para calcular distancia de Levenshtein.
    fn valid_words(&self) -> Vec<String>;
}