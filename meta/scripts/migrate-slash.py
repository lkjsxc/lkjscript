#!/usr/bin/env python3
"""Migrate attribute-less XML .lkjscript to slash grammar (structure only)."""
import re
import sys
from pathlib import Path

TAG = re.compile(r"<!--.*?-->|</([^\s>/]+)\s*>|<([^\s>/]+)\s*/>|<([^\s>/]+)\s*>|([^<]+)", re.S)

def migrate(src: str) -> str:
    out = []
    for m in TAG.finditer(src):
        if m.group(0).startswith("<!--"):
            continue
        if m.group(1) is not None:
            name = rename(m.group(1))
            out.append(f"/{name}")
        elif m.group(2) is not None:
            name = rename(m.group(2))
            out.append(name)
        elif m.group(3) is not None:
            name = rename(m.group(3))
            out.append(f"{name}/")
        else:
            text = m.group(4)
            if text is None:
                continue
            # preserve newlines as spacing; quote non-ws text nodes that aren't only whitespace
            if text.strip() == "":
                out.append("\n" if "\n" in text else " ")
                continue
            # XML text node -> string literal
            esc = (
                text.strip()
                .replace("\\", "\\\\")
                .replace('"', '\\"')
                .replace("\n", "\\n")
                .replace("\t", "\\t")
            )
            out.append(f'"{esc}"')
    # pretty-ish: ensure spaces between tokens
    s = " ".join(t for t in out if t != "")
    s = re.sub(r" +\n", "\n", s)
    s = re.sub(r"\n{3,}", "\n\n", s)
    return s.strip() + "\n"

def rename(name: str) -> str:
    if name == "/":
        return "div"
    return name

def main():
    roots = [Path("src"), Path("examples")]
    files = []
    for r in roots:
        if r.exists():
            files.extend(r.rglob("*.lkjscript"))
    for f in files:
        text = f.read_text()
        if "<" not in text:
            continue
        f.write_text(migrate(text))
        print("migrated", f)

if __name__ == "__main__":
    main()
