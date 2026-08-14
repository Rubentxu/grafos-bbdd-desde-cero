#!/usr/bin/env python3
"""Divide un manuscrito .md en ficheros por capítulo/sección.

Límites: encabezados de nivel 1 explícitamente reconocidos (título, Prólogo,
Tabla de contenidos, Sección, Capítulo N, Parte, Apéndice, Epílogo, Colofón).
Cualquier otro `# ...` (p.ej. `# Cargo.toml` dentro de bloques de capítulo) NO
es límite y permanece dentro del trozo anterior.

Genera: manuscrito/<vol>/<ficheros>.md + manuscrito/<vol>/SUMARIO.txt
El ensamblado por SUMARIO.txt debe reproducir el original byte a byte.
"""
import re
import sys
import unicodedata
from pathlib import Path

BOUNDARY = re.compile(
    r"^# (Capítulo (\d+)|Prólogo|Tabla de contenidos|Sección \d+|Parte [\w-]+|"
    r"Apéndice [\w0-9]+|Epílogo|Colofón|Grafos en Computación.*|Construye una base.*|Grafos en la era.*)"
)


def slugify(text: str) -> str:
    text = unicodedata.normalize("NFKD", text)
    text = "".join(c for c in text if not unicodedata.combining(c))
    text = re.sub(r"[^a-zA-Z0-9]+", "-", text).strip("-").lower()
    return re.sub(r"-{2,}", "-", text)[:60].strip("-")


def filename_for(header: str) -> str:
    h = header[2:].strip()
    m = re.match(r"Capítulo (\d+) — (.+)", h)
    if m:
        return f"cap-{int(m[1]):02d}-{slugify(m[2])}.md"
    if h.startswith("Prólogo"):
        return "prologo.md"
    if h.startswith("Tabla de contenidos"):
        return "tabla-de-contenidos.md"
    if h.startswith("Sección"):
        return slugify(h) + ".md"
    if h.startswith("Parte"):
        return slugify(h) + ".md"
    m = re.match(r"Apéndice ([A-Z0-9]+) — (.+)", h)
    if m:
        return f"apendice-{m[1].lower()}-{slugify(m[2])}.md"
    if h.startswith("Epílogo"):
        return "epilogo.md"
    if h.startswith("Colofón"):
        return "colofon.md"
    return "portada.md"  # título de portada del volumen


def split(src: Path, dest_dir: Path) -> None:
    lines = src.read_text(encoding="utf-8").splitlines(keepends=True)
    bounds: list[tuple[int, str]] = []  # (line_index, filename)
    for i, line in enumerate(lines):
        if BOUNDARY.match(line):
            bounds.append((i, filename_for(line.strip())))
    if not bounds:
        sys.exit(f"sin límites en {src}")
    dest_dir.mkdir(parents=True, exist_ok=True)
    names = []
    for k, (start, name) in enumerate(bounds):
        end = bounds[k + 1][0] if k + 1 < len(bounds) else len(lines)
        if k == 0 and start > 0:
            # lo que precede al primer límite (front-matter) va con la portada
            chunk = lines[:end]
        else:
            chunk = lines[start:end]
        (dest_dir / name).write_text("".join(chunk), encoding="utf-8")
        names.append(name)
    if len(names) != len(set(names)):
        dupes = {n for n in names if names.count(n) > 1}
        sys.exit(f"nombres duplicados en {src}: {dupes}")
    (dest_dir / "SUMARIO.txt").write_text(
        "\n".join(names) + "\n", encoding="utf-8"
    )
    print(f"{src.name}: {len(names)} ficheros -> {dest_dir}")


if __name__ == "__main__":
    root = Path("/var/home/rubentxu/Documentos/Libros-AI/grafos-bbdd-desde-cero")
    jobs = [
        ("vol1-grafos-de-cero-a-experto-rust.md", "vol1"),
        ("vol2-construye-liradb.md", "vol2"),
        ("vol3-grafos-era-ia.md", "vol3"),
    ]
    for fname, vol in jobs:
        split(root / fname, root / "manuscrito" / vol)
