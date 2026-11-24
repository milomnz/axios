# Memoria: árbol, generadores y validadores

Esta es mi parte del proyecto: va desde el árbol que entrega el analizador del
español hasta la traducción validada en Kethari y Ozh, y la lectura de regreso.
Las decisiones que el manual no resuelve están en `docs/bitacora.md`, cada una
con su prueba.

## 1. Recorrido de una pregunta por esta parte

```text
arbol (Ir) --> generador Kethari --> validador Kethari --> resultado
          \--> generador Ozh     --> validador Ozh     --> resultado
```

`translate_ir` (`src/backends/mod.rs`) recibe el árbol y devuelve un resultado
por lengua: la frase, si el validador la aceptó y el motivo cuando no. Una
lengua que falla no impide la otra. Cada paso queda en la traza.

## 2. Árbol de representación intermedia

Definido en `src/ir/mod.rs` y explicado en `docs/arbol_ir.md`. Describe una
pregunta o una oración sin palabras de ninguna lengua: todo se nombra con la
`semantic_key` de la tabla `concepts`.

| Elemento | Contenido |
|---|---|
| `SemanticSentence` | tiempo, negación, verbo, sujeto, objeto |
| `SemanticQuestion` | una oración más un interrogativo adjunto (cuándo, dónde) |
| `Argument` | pronombre, sintagma nominal o interrogativo en lugar del argumento |
| `NounPhrase` | sustantivo, cantidad, plural, poseedor |

## 3. Generadores

Los dos leen del léxico y de `grammar_rules`; nada de la lengua está en el
código. Si falta una regla o una palabra, devuelven un `GenError` que explica
el motivo, nunca inventan.

| | Kethari | Ozh |
|---|---|---|
| Orden | sujeto, objeto, verbo | verbo, sujeto, objeto |
| Tiempo | sufijo `-ok`, `-ime` | palabra al inicio, `ʔoka`, `venu` |
| Negación | prefijo `nu-` | `ne` antes del verbo |
| Plural | sufijo `-esh`, también con número | no hay; lo dice el número |
| Pregunta | `ka` al final | prefijo `ʔa-` en el verbo |
| Interrogativo | al inicio | al final |
| Poseedor | antes de lo poseído | después de lo poseído |
| Partícula evidencial | no tiene | obligatoria después del verbo |
| Números | base 6, cifras unidas con guion | base 10, cifras unidas con guion |

## 4. Gramáticas que leen los validadores

Kethari:

```bnf
<frase>     ::= [<interrog>] <sintagma>{0,2} <verbo> [ka]
<sintagma>  ::= <pronombre> | [<pronombre>] [<numero>] <sustantivo>
<verbo>     ::= [nu] <raiz> [ok | ime]
<sustantivo>::= <raiz> [esh]
```

Ozh:

```bnf
<frase>     ::= [ʔoka | venu] [ne] <verbo> <particula> <sintagma>{0,2} [<interrog>]
<sintagma>  ::= <pronombre> | [<numero>] <sustantivo> [<pronombre>]
<verbo>     ::= [ʔa] <raiz>
<particula> ::= zi | mar | tho
```

Condiciones que la gramática no muestra y el validador comprueba: un
interrogativo de sujeto u objeto deja sitio a un solo sintagma; sin
interrogativo hace falta sujeto; `ka` solo cierra y solo en preguntas; con el
número uno el sustantivo de Kethari va en singular y con cualquier otro en
plural; los números no empiezan con cero ni pasan de 9.999.

## 5. Análisis léxico de los validadores

Cada validador parte la frase en palabras con su posición y clasifica cada una
contra el vocabulario vigente de la base. Reconoce palabras completas y solo
separa los afijos que la gramática define (`nu-`, `-ok`, `-ime`, `-esh` en
Kethari; `ʔa-` en Ozh). No busca palabras dentro de otras, por eso `ʔenka`
(Ozh, "nosotros") no se lee como `ʔen` más `ka`. A diferencia del lexer del
español, no es un autómata carácter a carácter: no lo necesita porque las
palabras vienen separadas por espacios.

## 6. Análisis de errores

| Situación | Quién la detecta | Qué se dice |
|---|---|---|
| Palabra sin equivalente en la lengua | generador | `MissingWord`: la lengua no tiene palabra para el concepto |
| Concepto cuyo significado cambio (`teka`, semana 2) | generador | igual; solo se traduce lo que el manual respalda hoy |
| Oración sin sujeto | generador | `MissingArgument` |
| Regla ausente en la base | generador y validador | `MissingRule` |
| Palabra que el manual no respalda | validador | posición y palabra |
| Orden o forma incorrectos, `ka` o partícula mal puestos | validador | posición y motivo |
| Número mal formado | validador | cifra inexistente, cero a la izquierda o mayor que 9.999 |
| Salida rechazada por el validador | `translate_ir` | se guarda para auditar, marcada como no válida y no se transmite |

## 7. Ambigüedades

Un pronombre pegado a un sustantivo se lee de dos maneras (sujeto y objeto, o
poseedor y poseído). El validador entrega todas las lecturas válidas, la
preferida primero, en vez de elegir una a ciegas.

## 8. Pruebas

| Archivo | Qué cubre |
|---|---|
| `tests/numbers_tests.rs` | cifras en base 6 y base 10 contra las señales |
| `tests/kethari_tests.rs`, `tests/ozh_tests.rs` | cada generador contra las señales registradas y los casos sin traducción |
| `tests/validation_tests.rs` | las 20 señales con transcripción pasan; los casos inválidos se rechazan |
| `tests/pares_tests.rs` | 30 pares de pregunta y traducción escritos a mano, traza y recuperación del árbol |
| `tests/reverse_tests.rs` | lectura de Kethari y Ozh de regreso al español |

## 9. Uso de herramientas de inteligencia artificial

Pendiente: el enunciado pide declarar aquí el uso de herramientas de
inteligencia artificial. Falta que el equipo anote que herramientas uso, para
que consultas y en que partes del código.
