# Bitacora de decisiones

## 2026-11-20 · Integrante 1 · Como guardar los cambios de significado

**Problema.** El sistema debe incorporar correcciones del manual y responder que se creia en un momento que significaba cada palabra.

**Opciones.** (a) Actualizar la fila del lexico y guardar el valor viejo en una tabla de auditoria. (b) No modificar filas: cada fila tiene `valid_from` y `valid_to`, y una correccion cierra una y abre otra.

**Decision.** (b). Con (a), la consulta "que significaba en la fecha t" tiene que reconstruir el pasado desde la auditoria. Con (b) es un solo `WHERE valid_from <= t AND (valid_to IS NULL OR t < valid_to)`.

**Prueba.** `la_correccion_de_la_semana_2_versiona_teka` en `tests/db_tests.rs`, y `axios palabra teka en <fecha>`.

## 2026-11-20 · Integrante 1 · Desde cuando vale una correccion

**Problema.** La correccion de `teka` llega en el archivo de la semana 2, sin fecha propia. Las señales traen el dia (8, 9, 10) pero no una fecha del calendario.

**Opciones.** (a) Vigente desde el dia de la señal. (b) Vigente desde el momento en que se carga el archivo.

**Decision.** (b). Lo que se quiere auditar es que creia el sistema cuando tradujo algo, y antes de cargar el archivo el sistema creia que `teka` era "tierra". El "dia" de las señales es un numero de noche, no una fecha.

**Prueba.** `la_correccion_de_la_semana_2_versiona_teka`.

## 2026-11-20 · Integrante 1 · Que pasa con "tierra" en Kethari tras la correccion

**Problema.** Tras la correccion, `teka` significa "territorio reclamado". El manual no da otra palabra Kethari para "tierra".

**Opciones.** (a) Dejar `teka` tambien como "tierra". (b) "Tierra" se queda sin palabra en Kethari.

**Decision.** (b). La correccion dice que el significado anterior era un error; mantenerlo seria transmitir algo que el manual ya no respalda. Una pregunta con "tierra" no se traducira a Kethari y dira por que. En Ozh sigue siendo `mak`.

**Prueba.** `la_correccion_de_la_semana_2_versiona_teka` y `reaplicar_correcciones_o_recargar_el_manual_no_deshace_nada`.

## 2026-11-20 · Integrante 1 · Base numerica de Kethari

**Problema.** El manual solo da cifras Kethari del 0 al 5 y no dice como se forman los numeros mayores.

**Opciones.** (a) Base 10 como Ozh, sin poder escribir 6 a 9. (b) Base 6.

**Decision.** (b), porque es lo unico que explica las tres señales con numeros: `ra-sel` = 2·6+3 = 15 (dia 3), `ti-vo` = 1·6+0 = 6 (dia 7) y `mok-ra` = 4·6+2 = 26 (dia 10, que la semana 2 traduce como veintiseis). Se carga desde `data/deducciones_equipo.txt` como fuente `TEAM`, separada del manual.

**Prueba.** `el_manual_queda_en_la_base` comprueba la regla; la conversion la prueba el generador Kethari.

## 2026-11-20 · Integrante 1 · La señal desconocida del dia 6

**Problema.** "Ya los escuchamos." no tiene transcripcion ni pertenece a ninguna de las dos lenguas.

**Opciones.** (a) Descartarla. (b) Guardarla sin transcripcion y sin aportar palabras.

**Decision.** (b). Es una de las 15 señales registradas y debe quedar, pero no se sabe en que lengua esta ni que palabras tiene, asi que no respalda ninguna traduccion.

**Prueba.** `carga_las_15_senales_de_la_semana_1`.

## 2026-11-20 · Integrante 1 · anul y anir

**Problema.** Las dos se traducen "nosotros", pero el manual dice que linguistica no cree que sean sinonimos.

**Opciones.** (a) Marcar una como inclusiva y otra como exclusiva. (b) Guardar las dos para el mismo concepto con la nota del manual.

**Decision.** (b). Con las señales disponibles no se puede demostrar la diferencia, y la condicion 3 prohibe inventar. Que forma usa el generador lo decide el integrante 4 y lo anota aqui.

**Prueba.** `el_manual_queda_en_la_base`.

## 2026-11-20 · Integrante 1 · Hora de los registros

**Problema.** Hay que guardar cuando llego cada pregunta y cada archivo, y comparar fechas en `historial desde` y `palabra en`.

**Opciones.** (a) Hora UTC. (b) Hora local, con milisegundos, como texto `AAAA-MM-DD HH:MM:SS.mmm`.

**Decision.** (b). Quien consulta escribe fechas locales, y el texto en ese formato se ordena igual que el tiempo. Los milisegundos evitan empates entre una carga y una traduccion hechas en el mismo segundo. Una fecha sin hora vale como inicio del dia en `historial desde` y como final del dia en `palabra en`.

**Prueba.** `completa_fechas` en `src/db/mod.rs`.

## 2026-11-22 · Integrante 1 · Espanol sin tildes en la base

**Problema.** Las preguntas llegan muchas veces sin tildes, y en la base habia palabras con tilde ("cuando" con tilde, "vendran" con tilde). Comparar las dos formas obliga a quitar tildes en cada consulta.

**Opciones.** (a) Guardar con tildes y quitarlas al comparar. (b) Guardar el espanol sin tildes, igual que queda la pregunta despues del normalizador.

**Decision.** (b). La base y la salida del normalizador usan la misma forma, asi el corrector y la morfologia comparan directamente. Se pierde la diferencia entre "tu" pronombre y "tu" posesivo, "el" pronombre y "el" articulo, y "que" interrogativo y "que" conjuncion; esa diferencia la resuelve el analizador sintactico por la posicion en la oracion. La ñ se conserva, porque el enunciado la trata aparte de las tildes ("sin tildes, sin ñ") y "año" no es "ano". Las palabras de Kethari y Ozh no se tocan (ʔ y ü son parte de la transcripcion), ni los archivos de senales, que se cargan tal como los entrega Axios.

**Prueba.** `el_manual_queda_en_la_base` en `tests/db_tests.rs`.

## 2026-11-23 · Integrante 4 · Forma del árbol de representación intermedia

**Problema.** Los generadores necesitan el árbol del Integrante 3, que aún no existe. Esperar al parser bloquea el desarrollo de Kethari y Ozh.

**Opciones.** (a) Esperar a que el parser entregue su árbol. (b) Definir una versión inicial del árbol como contrato, desarrollar los generadores contra ella y dejar al Integrante 3 completarla.

**Decisión.** (b). El árbol nombra todo con `semantic_key` de la tabla `concepts`, para no depender de ninguna lengua, y separa el interrogativo que es argumento (`Argument::Wh`) del que no lo es (`adjunct`). Lo pendiente queda en `docs/arbol_ir.md`.

**Prueba.** Pruebas del módulo `ir` en `src/ir/mod.rs`.

## 2026-11-23 · Integrante 4 · Plural en el árbol

**Problema.** Kethari marca plural con `-esh` aunque haya número ("quince naves" -> `kumesh`). Ozh no marca plural nunca. El analizador podría marcar el plural gramatical o no.

**Opciones.** (a) Confiar solo en el campo `plural` del parser. (b) Que el generador también considere plural toda cantidad mayor que uno.

**Decisión.** (b), con `NounPhrase::is_plural()`. Así un parser que olvide marcar el plural no produce `kum` en lugar de `kumesh` en Kethari.

**Prueba.** `la_cantidad_mayor_que_uno_hace_plural` en `src/ir/mod.rs`.

## 2026-11-23 · Integrante 4 · De dónde sale la base numérica al generar números

**Problema.** Kethari cuenta en base 6 y Ozh en base 10. El generador necesita saber cual usar, que separador y como se llama cada cifra.

**Opciones.** (a) Dejar 6 y 10 como constantes en el generador. (b) Leer base, separador y cifras de la base de datos.

**Decisión.** (b). El enunciado prohibe escribir reglas dentro del código, y así una corrección futura del manual se aplica con `cargar` sin recompilar. Si falta una regla o una cifra, el generador devuelve un error en lugar de suponer un valor. El cero se escribe como una sola cifra (`vo`, `zu`) y el máximo es 9.999.

**Prueba.** `kethari_usa_base_6_como_en_las_senales`, `ozh_usa_base_10_como_en_las_senales` y `sin_reglas_en_la_base_no_se_inventa_nada` en `tests/numbers_tests.rs`.

## 2026-11-23 · Integrante 4 · Cuál forma de "nosotros" usa el generador de Kethari

**Problema.** El manual da `anul` y `anir` para "nosotros" y lingüistica no cree que sean sinónimos. En las señales, `anir` aparece en oraciones en pasado (días 1, 4 y 10) y `anul` en presente y futuro (días 2 y 7), pero son cinco casos y no hay una regla explícita.

**Opciones.** (a) Elegir según el tiempo verbal, imitando las señales. (b) Usar siempre la primera forma cargada.

**Decisión.** (b). Cinco observaciones no demuestran una regla, y una regla de ese tipo en el código sería inventar lo que el manual no dice. Las dos formas están respaldadas por señales, así que la que sale es válida; el validador de Kethari debe aceptar ambas. Si Axios aclara la diferencia, se cambia en la base.

**Prueba.** `dia_1_enviamos_mensajes_usa_anul_aunque_la_senal_diga_anir` y `dia_4_no_los_vimos_a_ustedes_lleva_nu_antes_del_verbo_y_ok_despues` en `tests/kethari_tests.rs`.

## 2026-11-23 · Integrante 4 · Orden de poseedor y cantidad en Kethari

**Problema.** El manual dice que el poseedor va delante de lo poseído y que los números van delante del sustantivo, pero no dice que va primero cuando hay ambos ("sus quince naves").

**Opciones.** (a) Poseedor, número, sustantivo. (b) Número, poseedor, sustantivo.

**Decisión.** (a), porque el poseedor se refiere al grupo completo y no al sustantivo solo. Ninguna señal registrada trae los dos a la vez, así que queda como supuesto a corregir si Axios publica un ejemplo.

**Prueba.** `el_poseedor_va_delante_de_lo_poseido` en `tests/kethari_tests.rs` cubre el poseedor solo.

## 2026-11-23 · Integrante 4 · Interrogativo de cantidad ("cuántos")

**Problema.** En "cuántas naves llegaron", la cantidad es lo que se pregunta. El árbol no trae un número y la palabra interrogativa debe ocupar su posición de interrogativo.

**Opciones.** (a) Dejar `cuantos` junto al sustantivo. (b) Tratarlo como cualquier otro interrogativo: sale al inicio en Kethari y al final en Ozh.

**Decisión.** (b), por la regla 3.3.7 (la palabra interrogativa va al principio). Ninguna señal lo confirma; es una aplicación de la regla general.

**Prueba.** `cuantas_naves_pone_el_interrogativo_al_inicio` en `tests/kethari_tests.rs`.

## 2026-11-23 · Integrante 4 · Qué partícula evidencial usa el generador de Ozh

**Problema.** Toda oración en Ozh lleva `zi` (lo vimos), `mar` (nos lo dijeron) o `tho` (lo deducimos) después del verbo. El español de los funcionarios no dice cómo se sabe lo que se pregunta, y el manual admite que no se sabe a qué se refiere la partícula en las preguntas (3.5). En las señales se reparten así: `zi` en los días 1, 4 y 8; `mar` en 5, 6 y 7; `tho` en 2, 3, 9 y 10, sin un patrón que permita predecir cuál va.

**Opciones.** (a) `zi`, que afirma haber visto. (b) `mar`, que afirma que otros lo dijeron. (c) `tho`, que solo dice que se dedujo. (d) Elegir por el tiempo o el sujeto, imitando las señales.

**Decisión.** (c) como valor por defecto, guardado en la base como `EVIDENTIAL_DEFAULT` y no en el código. Es la que menos afirma sobre lo que sabemos, y el director de Axios advirtió que una palabra equivocada puede sonar a amenaza. (d) se descartó porque los datos no sostienen una regla. `generate_with` permite elegir otra de las tres; cualquier partícula fuera del manual se rechaza.

**Prueba.** `la_particula_por_defecto_sale_de_la_base` y `una_particula_fuera_del_manual_se_rechaza` en `tests/ozh_tests.rs`. Los demás casos fijan la partícula de cada señal con `generate_with` para comprobar el resto de la gramática.

## 2026-11-23 · Integrante 4 · Pasado sin `ʔoka` en la señal del día 4

**Problema.** La señal del día 4 en Ozh, `ne ʔim zi ʔenka gor` ("No vimos ninguna nave"), está en pasado en español pero no lleva `ʔoka`, y el manual (3.4.3) dice que el pasado se marca con `ʔoka` al inicio. Las otras señales en pasado (días 1 y 8) sí lo llevan.

**Opciones.** (a) Seguir la señal y omitir `ʔoka` si hay negación. (b) Seguir la regla del manual y poner siempre `ʔoka` en pasado.

**Decisión.** (b). Una sola señal no basta para crear una excepción, y el decodificador "todavía se está calibrando". El generador produce `ʔoka ne ʔim zi ʔenka gor`; el validador de Ozh debe aceptar ambas formas al leer señales ajenas.

**Prueba.** `dia_4_en_pasado_el_manual_pide_oka_aunque_la_senal_no_lo_traiga` en `tests/ozh_tests.rs`.

## 2026-11-23 · Integrante 4 · Orden de cantidad, sustantivo y poseedor en Ozh

**Problema.** Ozh pone el número delante del sustantivo y el poseedor detrás, y el manual no dice cómo se combinan.

**Opciones.** (a) Número, sustantivo, poseedor. (b) Sustantivo, poseedor, número.

**Decisión.** (a), porque respeta las dos reglas sin mover ninguna. Ninguna señal trae ambos a la vez; queda como supuesto.

**Prueba.** `dia_8_visitaron_nuestro_mundo_pone_el_poseedor_detras` cubre el poseedor; el orden con ambos no tiene señal que lo confirme.

## 2026-11-23 · Integrante 4 · Cómo lee el validador lo que generó el generador

**Problema.** El enunciado pide que el validador sea un analizador independiente y que rechace frases inválidas. Si usará el código de los generadores, un error de los generadores pasaría como válido.

**Opciones.** (a) Comparar la frase con la que produciría el generador. (b) Un lexer y un parser propios por lengua, que solo comparten con los generadores la lectura de la base.

**Decisión.** (b). El lexer de cada lengua reconoce palabras completas contra el vocabulario vigente y solo parte los afijos que la gramática define (`nu-`, `-ok`, `-ime`, `-esh` en Kethari; `ʔa-` en Ozh). Las palabras no se parten buscando otras dentro, así que `ʔenka` en Ozh no se lee como `ʔen` más `ka`. El parser comprueba la forma de la frase según la gramática documentada al inicio de `src/validation/kethari.rs` y `src/validation/ozh.rs`. Todo lo que no reconoce se rechaza con la posición del problema.

**Prueba.** `acepta_todas_las_senales_registradas_de_kethari`, `acepta_todas_las_senales_registradas_de_ozh` y `lee_lo_que_generan_los_generadores` en `tests/validation_tests.rs`; el resto son casos que deben rechazarse.

## 2026-11-23 · Integrante 4 · Qué acepta el validador que el generador no produce

**Problema.** Las señales reales no siempre siguen al pie de la letra las reglas del manual (el día 4 de Ozh no trae `ʔoka`; `anul` y `anir` se alternan). Un validador demasiado estricto rechazaría lo que las propias voces dijeron.

**Opciones.** (a) Validar exactamente lo que el generador produce. (b) Aceptar todo lo que la gramática permite y las señales muestran.

**Decisión.** (b). El validador acepta `ʔoka` ausente, las dos formas de "nosotros" y las tres partículas evidenciales, y las 20 señales con transcripción pasan. Rechaza lo que el manual no respalda: palabras desconocidas, orden incorrecto, `ka` fuera de lugar, partícula evidencial ausente, números mayores que uno con sustantivo singular y cifras mal formadas.

**Prueba.** Las dos pruebas que recorren los archivos de señales, y las de rechazo en `tests/validation_tests.rs`.

## 2026-11-24 · Integrante 4 · Cómo se reúnen generador y validador

**Problema.** La orden `traducir` necesita, para cada lengua, la traducción, el resultado del validador y el motivo cuando algo falla. Si las dos lenguas se tratan juntas, un error en una impediría transmitir la otra.

**Opciones.** (a) Fallar toda la pregunta si una lengua falla. (b) Cada lengua con su propio resultado.

**Decisión.** (b), con `translate_ir` en `src/backends/mod.rs`. Una salida que el validador rechaza se conserva con su motivo pero se marca como no válida, para que no se transmita y quede auditable en el historial. La traza anota generador y validador de cada lengua. La condición 1 de Axios se cumple igual: siempre hay respuesta, con la traducción o con una explicación.

**Prueba.** `una_lengua_sin_palabra_no_bloquea_a_la_otra_y_explica_el_motivo` y `la_traza_anota_generador_y_validador_de_cada_lengua` en `tests/pares_tests.rs`.

## 2026-11-24 · Integrante 4 · Frases ambiguas al leer Kethari y Ozh

**Problema.** Al leer una frase, un pronombre pegado a un sustantivo admite dos lecturas. En Kethari, `anul parn` es "nosotros" y "paz" (sujeto y objeto) o "nuestra paz" (poseedor y poseído); en Ozh pasa lo mismo con un pronombre después de un sustantivo. La gramática del manual no lo resuelve y ningún verbo trae marcada su transitividad.

**Opciones.** (a) Elegir una lectura por regla fija. (b) Devolver todas las lecturas válidas, con la preferida primero.

**Decisión.** (b). El validador sigue aceptando la frase si alguna lectura es válida. `Reading.ir` trae la preferida (la que separa sujeto y objeto) y `Reading.alternatives` las demás. Así no se descarta una lectura que podría ser la correcta, y quien use el árbol decide. En las pruebas de pares, el árbol original debe estar entre las lecturas.

**Prueba.** `los_validadores_recuperan_el_arbol_de_cada_par_entre_sus_lecturas` en `tests/pares_tests.rs` y `una_frase_ambigua_devuelve_las_dos_lecturas` en `tests/reverse_tests.rs`.

## 2026-11-23 · Integrante 4 · Sustantivo seguido de pronombre en Ozh

**Problema.** En Ozh el poseedor va detrás de lo poseído, así que `thüm ʔenka` es "nuestro mundo". Pero un sustantivo sujeto seguido de un pronombre objeto tendría la misma forma.

**Opciones.** (a) Leerlo siempre como poseedor. (b) Intentar las dos lecturas.

**Decisión.** (a). Con el orden verbo, sujeto, objeto, las dos lecturas son frases válidas, y el validador solo debe decidir si la frase es válida, no cuál de ellas fue la intención. Si se hace la traducción de regreso al español (bonus), esta ambigüedad habrá que resolverla ahí.

**Prueba.** `dia_8_visitaron_nuestro_mundo_pone_el_poseedor_detras` y `acepta_todas_las_senales_registradas_de_ozh`.
