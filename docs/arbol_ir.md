# Árbol de representación intermedia (IR)

Contrato entre el analizador sintáctico del español (Integrante 3) y los
generadores de Kethari y Ozh (Integrante 4). Código en `src/ir/mod.rs`.

Es una primera versión que dejé armada para poder avanzar con los
generadores sin esperar al parser. Quien lo termine (Integrante 3) la completa
y la ajusta. Si cambia la forma del árbol, avisar al Integrante 4 y anotarlo
en la bitácora.

## Qué hay hecho

- `Ir`: `Sentence(SemanticSentence)` o `Question(SemanticQuestion)`.
- `SemanticSentence`: tiempo, negación, verbo, sujeto y objeto.
- `SemanticQuestion`: una oración más un interrogativo adjunto ("cuándo", "dónde").
- `Argument`: pronombre, sintagma nominal o interrogativo en lugar del argumento.
- `NounPhrase`: sustantivo, cantidad, plural y poseedor.
- Todo se nombra con la `semantic_key` de la tabla `concepts`, no con palabras.

## Reglas del contrato

1. El sujeto es obligatorio en las dos lenguas. Si el español lo omite
   ("¿Cuándo llegan?"), el analizador lo completa desde la persona del verbo
   (tercera persona del plural -> `ELLOS`).
2. `ir + a + infinitivo` llega como `Tense::Future`. Los generadores no ven la
   perífrasis.
3. Un interrogativo que es sujeto u objeto ("quién envió…", "qué quieren")
   va en su lugar como `Argument::Wh`. Uno que no lo es ("cuándo", "dónde")
   va en `adjunct`. Los generadores lo mueven a su posición.
4. Pregunta de sí o no: ni `adjunct`, ni `Wh`, ni cantidad `Unknown`.
5. `NounPhrase::is_plural()` es verdadero con `plural` o con una cantidad
   mayor que uno. Kethari pone `-esh` incluso con número; Ozh nunca marca plural.
6. El complemento de "ser" ("Es el una criatura") va en `object`.

## Lo que falta por completar (Integrante 3)

- Producir el árbol desde los tokens del corrector, con el parser descendente
  recursivo y la BNF documentada.
- Completar el sujeto omitido a partir de la morfología verbal.
- Decidir dónde queda un posesivo cuando el poseedor es un sustantivo
  ("el planeta de ellos"). Hoy solo se admite un pronombre.
- Preguntas con "cuántos" sobre el sujeto y sobre el objeto: verificar que la
  forma `Quantity::Unknown` alcanza.
- Oraciones con más de un verbo, o con complementos que no sean sujeto ni
  objeto (lugar, tiempo). No están en las 21 señales, pero pueden llegar en
  las preguntas de la sustentación.
- Quién es el sujeto de la oración cuando llega una pregunta con "a quién"
  ("¿A quién escuchan ustedes?" -> `QUIEN` como objeto).
- Números: el normalizador entrega el valor entero; la conversión a cifras de
  cada lengua la hace el generador.

## Ejemplos

"Quince naves llegaron" ->
`Sentence { tense: Past, verb: VENIR, subject: Noun(NAVE x15), object: None }`

"¿Cuándo llegan?" ->
`Question { sentence: { Present, VENIR, subject: ELLOS }, adjunct: CUANDO }`
