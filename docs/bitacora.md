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
