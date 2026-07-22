#!/usr/bin/env python3
"""Convert legacy whitespace-separated .lkjscript files to canonical LKJML."""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path

IDENT = frozenset("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-+*=!?<>.:")


@dataclass(frozen=True)
class Token:
    kind: str
    value: str


def tokenize(source: str) -> list[Token]:
    tokens: list[Token] = []
    i = 0
    while i < len(source):
        if source[i].isspace():
            i += 1
            continue
        if source.startswith(";;", i):
            end = source.find("\n", i)
            end = len(source) if end < 0 else end
            tokens.append(Token("comment", source[i:end]))
            i = end
            continue
        if source[i] == '"':
            value, i = read_string(source, i)
            tokens.append(Token("string", value))
            continue
        if source[i] == "/":
            value, i = read_ident(source, i + 1)
            if not value:
                raise ValueError("close marker needs a name after /")
            tokens.append(Token("close", value))
            continue
        value, next_i = read_ident(source, i)
        if not value:
            raise ValueError(f"unexpected character {source[i]!r} at byte {i}")
        if next_i < len(source) and source[next_i] == "/":
            tokens.append(Token("open", value))
            i = next_i + 1
        else:
            tokens.append(Token("atom", value))
            i = next_i
    return tokens


def read_ident(source: str, start: int) -> tuple[str, int]:
    i = start
    while i < len(source) and source[i] in IDENT:
        i += 1
    return source[start:i], i


def read_string(source: str, start: int) -> tuple[str, int]:
    out: list[str] = []
    i = start + 1
    escapes = {"\\": "\\", '"': '"', "n": "\n", "t": "\t"}
    while i < len(source):
        char = source[i]
        if char == '"':
            return "".join(out), i + 1
        if char == "\\":
            i += 1
            if i >= len(source) or source[i] not in escapes:
                raise ValueError(f"invalid string escape at byte {i}")
            out.append(escapes[source[i]])
        else:
            out.append(char)
        i += 1
    raise ValueError("unterminated string")


def convert(source: str) -> str:
    lines: list[str] = []
    stack: list[str] = []
    for token in tokenize(source):
        if token.kind == "comment":
            lines.append(token.value)
        elif token.kind == "open":
            lines.append(f"{token.value}/")
            stack.append(token.value)
        elif token.kind == "close":
            if not stack or stack[-1] != token.value:
                expected = stack[-1] if stack else "end of file"
                raise ValueError(f"mismatched /{token.value}; expected /{expected}")
            lines.append(f"/{token.value}")
            stack.pop()
        elif token.kind == "atom":
            lines.append(token.value)
        elif stack and stack[-1] in {"name", "import"}:
            if not token.value or "\n" in token.value or token.value.strip() != token.value:
                raise ValueError(f"{stack[-1]}/ needs one non-empty trimmed text line")
            lines.append(token.value.replace(".lkjscript", ".lkjml"))
        else:
            lines.append("str/")
            if token.value:
                for text_line in token.value.split("\n"):
                    lines.append("\\/str" if text_line == "/str" else text_line)
            lines.append("/str")
    if stack:
        raise ValueError(f"unclosed {stack[-1]}/")
    return "\n".join(lines) + "\n"


def source_files(paths: list[Path]) -> list[Path]:
    files: list[Path] = []
    for path in paths:
        if path.is_dir():
            files.extend(sorted(path.rglob("*.lkjscript")))
        elif path.suffix == ".lkjscript":
            files.append(path)
        else:
            raise ValueError(f"expected a .lkjscript file or directory: {path}")
    return sorted(set(files))


def migrate(path: Path) -> Path:
    destination = path.with_suffix(".lkjml")
    if destination.exists():
        raise FileExistsError(destination)
    destination.write_text(convert(path.read_text()))
    destination.chmod(path.stat().st_mode)
    path.unlink()
    return destination


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("paths", nargs="+", type=Path)
    args = parser.parse_args()
    for path in source_files(args.paths):
        destination = migrate(path)
        print(f"{path} -> {destination}")


if __name__ == "__main__":
    main()
