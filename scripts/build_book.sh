#!/usr/bin/env bash
# Ensambla los manuscritos desde manuscrito/volN/ (ficheros en el orden de SUMARIO.txt)
# hacia los ficheros completos de la raíz del repo.
#
# Uso:
#   scripts/build_book.sh            # ensambla los 3 volúmenes
#   scripts/build_book.sh --check    # además verifica que los ensamblados commiteados
#                                    # están sincronizados (exit 1 si divergen)
set -euo pipefail

ROOT="$(cd "$(dirname "$BASH_SOURCE")/.." && pwd)"

declare -A SALIDA=(
  [vol1]="vol1-grafos-de-cero-a-experto-rust.md"
  [vol2]="vol2-construye-liradb.md"
  [vol3]="vol3-grafos-era-ia.md"
)

fallos=0
for vol in vol1 vol2 vol3; do
  dir="$ROOT/manuscrito/$vol"
  sumario="$dir/SUMARIO.txt"
  destino="$ROOT/${SALIDA[$vol]}"

  [ -f "$sumario" ] || { echo "FALTA $sumario" >&2; exit 2; }

  : > "$destino"
  while IFS= read -r fichero; do
    case "$fichero" in ""|\#*) continue ;; esac
    origen="$dir/$fichero"
    [ -f "$origen" ] || { echo "FALTA $origen (citado en SUMARIO.txt)" >&2; exit 2; }
    cat "$origen" >> "$destino"
  done < "$sumario"
  echo "✓ $vol: $(grep -c '' "$destino") líneas -> ${SALIDA[$vol]}"

  if [ "${1:-}" = "--check" ]; then
    if ! git -C "$ROOT" diff --quiet -- "$destino"; then
      echo "✗ $vol: ${SALIDA[$vol]} NO está sincronizado con manuscrito/$vol (ejecuta scripts/build_book.sh y commitea)" >&2
      fallos=1
    fi
  fi
done

[ "$fallos" -eq 0 ] || exit 1
echo "ALL_ASSEMBLED"
