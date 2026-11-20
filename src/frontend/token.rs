#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Word,               // Palabras (verbos, sustantivos, pronombres, etc.)
    Number(u16),        // Valores numéricos resueltos (0 a 9999)
    QuestionMarkOpen,   // ¿
    QuestionMarkClose,  // ?
    Eof,                // Fin de entrada
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub lexeme: String,
    pub span: Span,
    pub was_corrected: bool, // Útil para el modo traza e historial de correcciones
}