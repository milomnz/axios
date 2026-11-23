# Traductor Axios

Traduce preguntas en español a Kethari y Ozh. Está escrito en Rust y los datos viven en SQLite.

## Requisitos

Rust con `cargo` (https://rustup.rs). No hace falta instalar SQLite: viene incluido en la dependencia `rusqlite`.

Todos los comandos se ejecutan desde la raíz del repositorio.

## Compilar

```bash
cargo build --release
```

El ejecutable queda en `./target/release/axios`.

## Crear la base inicial

```bash
./crear_base.sh
```

El script compila el programa, borra `axios.db` si existe y la vuelve a crear con el manual v0.7, las reglas deducidas por el equipo, los datos del español y las 15 señales de la semana 1.

Es lo mismo que ejecutar a mano:

```bash
./target/release/axios cargar data/manual_v0.7.txt
./target/release/axios cargar data/deducciones_equipo.txt
./target/release/axios cargar data/espanol_base.txt
./target/release/axios cargar data/senales_semana1.txt
```

La base queda en `axios.db`, en la carpeta actual. Para usar otra ruta: `AXIOS_DB=otra.db ./target/release/axios ...`.

## Órdenes

```bash
./target/release/axios traducir "¿Cuándo llegan?"
./target/release/axios traducir --traza "cuando yegan"
./target/release/axios cargar data/senales_semana2.txt
./target/release/axios historial
./target/release/axios historial desde 2026-11-20
./target/release/axios palabra teka
./target/release/axios palabra teka en "2026-11-20 10:30"
```

| Orden | Qué hace |
|---|---|
| `traducir "<pregunta>"` | Traduce a Kethari y Ozh y lo registra todo. Con `--traza` muestra la salida de cada fase |
| `cargar <archivo>` | Incorpora un archivo de señales, correcciones o léxico, sin recompilar |
| `historial [desde <fecha>]` | Lista lo traducido y con qué resultado |
| `palabra <término> [en <fecha>]` | Muestra lo que el sistema sabe de una palabra en las tres lenguas, hoy o en esa fecha |

Las fechas se escriben `AAAA-MM-DD` o `AAAA-MM-DD HH:MM`. La pregunta va entre comillas: sin ellas, la terminal toma el `?` como comodín.

## Formato de los archivos de datos

Una línea por dato, con los campos separados por `|`. Las líneas que empiezan por `#` son comentarios. Los tipos de línea (`SENAL`, `CORRECCION`, `LEXICO`, `REGLA`…) están descritos al principio de `src/db/loader.rs`.

Si una línea es inválida no se carga nada del archivo, y el error indica el número de línea. Cargar dos veces el mismo archivo no duplica datos.

## Pruebas

```bash
cargo test
```

## Documentación

- `docs/modelo_datos.md`: diagrama entidad-relación y justificación del modelo de datos.
- `docs/bitacora.md`: decisiones fechadas, con las alternativas y la prueba de cada una.
