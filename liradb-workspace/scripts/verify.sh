#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────
# verify.sh — Verificador de calidad del workspace liradb-workspace
#
# Ejecuta la cadena de comandos definida en planning/stack-profile.yml
# sobre el workspace Rust actual. Estado de salida:
#
#   0 = ALL_GREEN   (fmt, check, test, lint todos OK)
#   1 = BLOCKED     (algún comando falló; ver stderr y report.jsonl)
#   2 = STACK_PROFILE_NOT_FOUND
#   3 = LENGUAJE_NO_SOPORTADO
#
# Uso:
#   ./scripts/verify.sh                  # cadena completa
#   ./scripts/verify.sh --verbose        # con output completo de cargo
#   ./scripts/verify.sh --skip=lint      # saltar lint (debugging)
#
# El script es idempotente y sin side-effects: NO modifica nada,
# sólo lee y ejecuta `cargo`. Produce build/verify-report.jsonl
# como side-effect controlado (registro de auditoría).
#
# Basado en ~/.zcode/skills/code-example-verifier/assets/run-stack-verify.sh
# ─────────────────────────────────────────────────────────────────

set -uo pipefail

# ────────────── Configuración ──────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO="$(cd "$SCRIPT_DIR/.." && pwd)"
STACK_PROFILE="$REPO/planning/stack-profile.yml"
REPORT_DIR="$REPO/build"
REPORT_FILE="$REPORT_DIR/verify-report.jsonl"
VERBOSE=0
SKIP_LINT=0
SKIP_FMT=0
SKIP_CHECK=0
SKIP_TEST=0

# Parseo de flags
for arg in "$@"; do
    case "$arg" in
        --verbose|-v) VERBOSE=1 ;;
        --skip=lint)  SKIP_LINT=1 ;;
        --skip=fmt)   SKIP_FMT=1 ;;
        --skip=check) SKIP_CHECK=1 ;;
        --skip=test)  SKIP_TEST=1 ;;
        --help|-h)
            sed -n '2,18p' "$0"
            exit 0
            ;;
        *)
            echo "verify.sh: argumento desconocido '$arg'" >&2
            exit 64
            ;;
    esac
done

# ────────────── Pre-checks ──────────────
if [ ! -f "$STACK_PROFILE" ]; then
    echo "BLOCKED: stack-profile no encontrado en $STACK_PROFILE" >&2
    exit 2
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "BLOCKED: 'cargo' no está en PATH. Instala Rust stable." >&2
    exit 2
fi

if ! command -v python3 >/dev/null 2>&1; then
    echo "BLOCKED: 'python3' no está en PATH (necesario para parsear YAML)." >&2
    exit 2
fi

mkdir -p "$REPORT_DIR"
TIMESTAMP="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

# ────────────── Helpers ──────────────

# Imprime el mensaje de workspace vacío y sale con código 0 (verde).
# El verificador considera "vacío y bootstrap completo" como estado válido.
print_empty_state() {
    local reason="$1"
    echo ""
    echo "═══════════════════════════════════════════════"
    echo "  WORKSPACE_EMPTY — ${reason}"
    echo "═══════════════════════════════════════════════"
    echo "Aún no se han añadido crates al workspace."
    echo "Para migrar el primer capítulo:"
    echo "  1. Crea crates/vol<N>-cap-NN-<slug>/ con su Cargo.toml + src/lib.rs"
    echo "  2. Decláralo en Cargo.toml [workspace] members"
    echo "  3. Vuelve a ejecutar ./scripts/verify.sh"
    echo "Salida temprana: ALL_GREEN (no hay nada que verificar todavía)."
    exit 0
}

# Parsea una clave con notación de puntos del YAML.
# Soporta "primary.fmt_tool" recorriendo la jerarquía stack.primary.fmt_tool.
# Usa PyYAML si está disponible; si no, parser propio tolerante.
parse_yaml() {
    local key="$1"
    python3 - "$STACK_PROFILE" "$key" <<'PY'
import sys
path, dotted_key = sys.argv[1], sys.argv[2]
# Si la clave no empieza por 'stack.', pre-añadir 'stack.' (convención del schema).
# Esto permite pedir 'primary.language' y que se resuelva como 'stack.primary.language'.
parts = dotted_key.split('.')
if parts and parts[0] != 'stack':
    parts = ['stack'] + parts

def try_yaml():
    try:
        import yaml
        with open(path) as f:
            doc = yaml.safe_load(f)
        node = doc
        for p in parts:
            if not isinstance(node, dict) or p not in node:
                return None
            node = node[p]
        return str(node).strip() if node is not None else None
    except ImportError:
        return None
    except Exception:
        return None

def try_fallback():
    """Parser YAML tolerante para el subconjunto que usamos (clave: valor)."""
    try:
        with open(path) as f:
            lines = f.readlines()
    except Exception:
        return None
    # Construir un dict con notación de puntos a partir de la indentación.
    flat = {}
    parent_stack = [(-1, '')]  # (indent_level, dotted_path)
    for raw in lines:
        line = raw.rstrip('\n')
        if not line.strip() or line.lstrip().startswith('#'):
            continue
        stripped = line.lstrip(' ')
        indent = len(line) - len(stripped)
        if ':' not in stripped:
            continue
        # Pop niveles más profundos o iguales que el actual
        while parent_stack and parent_stack[-1][0] >= indent:
            parent_stack.pop()
        if not parent_stack:
            continue
        key_part, _, val_part = stripped.partition(':')
        key_part = key_part.strip()
        val_part = val_part.strip()
        if val_part.startswith('"') and val_part.endswith('"'):
            val_part = val_part[1:-1]
        elif val_part.startswith("'") and val_part.endswith("'"):
            val_part = val_part[1:-1]
        parent_path = parent_stack[-1][1]
        full_key = f"{parent_path}.{key_part}" if parent_path else key_part
        flat[full_key] = val_part
        if not val_part:
            parent_stack.append((indent, full_key))
    return flat.get(dotted_key)

result = try_yaml() or try_fallback()
if result is not None:
    print(result)
PY
}

# Ejecuta un comando y registra resultado en build/verify-report.jsonl.
# Args: <step_name> <command_string>
run_step() {
    local step_name="$1"
    local cmd="$2"
    local start_ts end_ts duration_ms exit_code output

    echo ""
    echo "==> $step_name: $cmd"

    start_ts="$(date +%s%N)"
    if [ "$VERBOSE" -eq 1 ]; then
        output="$(bash -c "$cmd" 2>&1)"
        exit_code=$?
    else
        output="$(bash -c "$cmd" 2>&1 | tail -50)"
        exit_code=${PIPESTATUS[0]}
    fi
    end_ts="$(date +%s%N)"
    duration_ms=$(( (end_ts - start_ts) / 1000000 ))

    # Append al report (JSONL)
    python3 - "$REPORT_FILE" "$TIMESTAMP" "$step_name" "$cmd" "$exit_code" "$duration_ms" "$output" <<'PY' 2>/dev/null || true
import json, sys
report_path, ts, step, cmd, ec, dur, out = sys.argv[1:8]
try:
    ec_int = int(ec)
except ValueError:
    ec_int = -1
try:
    dur_int = int(dur)
except ValueError:
    dur_int = -1
entry = {
    "timestamp": ts,
    "step": step,
    "command": cmd,
    "exit_code": ec_int,
    "duration_ms": dur_int,
    "output_tail": out[-4000:] if len(out) > 4000 else out,
}
with open(report_path, "a") as f:
    f.write(json.dumps(entry, ensure_ascii=False) + "\n")
PY

    if [ "$exit_code" -ne 0 ]; then
        echo "✗ $step_name FAILED (exit=$exit_code)" >&2
        if [ "$VERBOSE" -eq 0 ] && [ -n "$output" ]; then
            echo "--- output (last 50 lines) ---" >&2
            echo "$output" >&2
            echo "--- end output ---" >&2
        fi
        return 1
    fi
    echo "✓ $step_name OK (${duration_ms}ms)"
    return 0
}

# ────────────── Detección de stack ──────────────
LANGUAGE="$(parse_yaml 'primary.language')"
case "$LANGUAGE" in
    rust)
        echo "==> Stack detectado: Rust (cargo)"
        echo "==> Workspace: $REPO"
        echo "==> Toolchain: $(rustc --version 2>&1 | head -1)"
        ;;
    *)
        echo "BLOCKED: lenguaje '$LANGUAGE' no soportado por el runner." >&2
        echo "Amplía verify.sh con las recetas de book-stack-detector." >&2
        exit 3
        ;;
esac

cd "$REPO" || exit 2

# Si el workspace está vacío (sin miembros efectivos en Cargo.toml),
# emitir un report de "skip" verde y salir. Consideramos "vacío" si:
#   - no hay línea `members = ...` en absoluto, o
#   - la lista de miembros está vacía o sólo contiene comentarios.
if ! grep -qE '^\s*members\s*=' Cargo.toml 2>/dev/null; then
    EMPTY_REASON="no hay miembros declarados en [workspace]"
elif python3 -c "
import re, sys
with open('Cargo.toml') as f:
    content = f.read()
m = re.search(r'^\s*members\s*=\s*\[(.*?)\]', content, re.MULTILINE | re.DOTALL)
if not m:
    sys.exit(1)
body = m.group(1)
entries = [l for l in body.split('\n')
           if l.strip() and not l.strip().startswith('#')]
sys.exit(0 if not entries else 1)
"; then
    EMPTY_REASON="todos los miembros están comentados"
fi

if [ -n "${EMPTY_REASON:-}" ]; then
    # Generar entrada de report trazable (skip verde)
    mkdir -p "$REPORT_DIR"
    > "$REPORT_FILE"
    python3 - "$REPORT_FILE" "$TIMESTAMP" "$EMPTY_REASON" <<'PY'
import json, sys
report_path, ts, reason = sys.argv[1], sys.argv[2], sys.argv[3]
entry = {
    "timestamp": ts,
    "step": "workspace_empty_check",
    "command": "(short-circuit)",
    "exit_code": 0,
    "duration_ms": 0,
    "output_tail": f"WORKSPACE_EMPTY: {reason}. No hay crates miembros aún; bootstrap completo OK.",
}
with open(report_path, "a") as f:
    f.write(json.dumps(entry, ensure_ascii=False) + "\n")
PY
    echo ""
    echo "═══════════════════════════════════════════════"
    echo "  WORKSPACE_EMPTY — ${EMPTY_REASON}"
    echo "═══════════════════════════════════════════════"
    echo "Aún no se han añadido crates al workspace."
    echo "Para migrar el primer capítulo:"
    echo "  1. Crea crates/vol<N>-cap-NN-<slug>/ con su Cargo.toml + src/lib.rs"
    echo "  2. Decláralo en Cargo.toml [workspace] members"
    echo "  3. Vuelve a ejecutar ./scripts/verify.sh"
    echo "Salida temprana: ALL_GREEN (no hay nada que verificar todavía)."
    exit 0
fi

# ────────────── Cadena de verificación ──────────────
# Limpiar report previo (un run = un report fresco).
> "$REPORT_FILE"

FAILED=0

if [ "$SKIP_FMT" -eq 0 ]; then
    FMT_CMD="$(parse_yaml 'primary.fmt_tool')"
    run_step "fmt_check" "$FMT_CMD" || FAILED=1
fi

if [ "$SKIP_CHECK" -eq 0 ] && [ "$FAILED" -eq 0 ]; then
    CHECK_CMD="$(parse_yaml 'primary.build_tool')"
    run_step "build_check" "$CHECK_CMD" || FAILED=1
fi

if [ "$SKIP_TEST" -eq 0 ] && [ "$FAILED" -eq 0 ]; then
    TEST_CMD="$(parse_yaml 'primary.test_runner')"
    run_step "tests" "$TEST_CMD" || FAILED=1
fi

if [ "$SKIP_LINT" -eq 0 ] && [ "$FAILED" -eq 0 ]; then
    LINT_CMD="$(parse_yaml 'primary.lint_tool')"
    run_step "lint_clippy" "$LINT_CMD" || FAILED=1
fi

# ────────────── Veredicto ──────────────
echo ""
if [ "$FAILED" -eq 0 ]; then
    echo "═══════════════════════════════════════════════"
    echo "  ALL_GREEN — el workspace pasa fmt+check+test+lint"
    echo "═══════════════════════════════════════════════"
    echo "Reporte: $REPORT_FILE"
    exit 0
else
    echo "═══════════════════════════════════════════════"
    echo "  BLOCKED — la cadena de verificación ha fallado"
    echo "═══════════════════════════════════════════════"
    echo "Reporte: $REPORT_FILE (revisar para detalle)"
    exit 1
fi