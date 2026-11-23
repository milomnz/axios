#!/bin/sh
# Crea axios.db desde cero con el manual, las deducciones, el español y las señales de la semana 1.
set -e
cd "$(dirname "$0")"
cargo build --release --quiet
rm -f axios.db
for f in manual_v0.7 deducciones_equipo espanol_base senales_semana1; do
  ./target/release/axios cargar "data/$f.txt"
done
