#!/usr/bin/env python3
from __future__ import annotations

import ast
import re
from collections import OrderedDict
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SRC_DIR = ROOT / "src"
POT_PATH = ROOT / "po" / "io.github.basshift.Recall.pot"

TR_CALL_RE = re.compile(r"\btr\(\s*\"", re.MULTILINE)


def line_number_for_offset(text: str, offset: int) -> int:
    return text.count("\n", 0, offset) + 1


def decode_rust_string_literal(source: str) -> str:
    return ast.literal_eval(source)


def extract_messages(path: Path) -> list[tuple[int, str]]:
    text = path.read_text(encoding="utf-8")
    messages: list[tuple[int, str]] = []
    for match in TR_CALL_RE.finditer(text):
        start = match.end() - 1
        escaped = False
        index = start + 1
        while index < len(text):
            char = text[index]
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                literal = text[start : index + 1]
                try:
                    message = decode_rust_string_literal(literal)
                except (SyntaxError, ValueError) as exc:
                    rel_path = path.relative_to(ROOT)
                    raise SystemExit(f"failed to decode string literal in {rel_path}:{line_number_for_offset(text, start)}: {exc}")
                messages.append((line_number_for_offset(text, match.start()), message))
                break
            index += 1
        else:
            rel_path = path.relative_to(ROOT)
            raise SystemExit(f"unterminated tr() string in {rel_path}:{line_number_for_offset(text, match.start())}")
    return messages


def escape_po(value: str) -> str:
    return (
        value.replace("\\", "\\\\")
        .replace('"', '\\"')
        .replace("\n", "\\n")
    )


def write_pot(messages: OrderedDict[str, list[str]]) -> None:
    header = [
        'msgid ""',
        'msgstr ""',
        '"Project-Id-Version: Recall\\n"',
        '"Report-Msgid-Bugs-To: \\n"',
        '"POT-Creation-Date: YEAR-MO-DA HO:MI+ZONE\\n"',
        '"PO-Revision-Date: YEAR-MO-DA HO:MI+ZONE\\n"',
        '"Last-Translator: FULL NAME <EMAIL@ADDRESS>\\n"',
        '"Language-Team: LANGUAGE <LL@li.org>\\n"',
        '"Language: \\n"',
        '"MIME-Version: 1.0\\n"',
        '"Content-Type: text/plain; charset=UTF-8\\n"',
        '"Content-Transfer-Encoding: 8bit\\n"',
        "",
    ]

    lines = header
    for message, refs in messages.items():
        lines.append(f"#: {' '.join(refs)}")
        lines.append(f'msgid "{escape_po(message)}"')
        lines.append('msgstr ""')
        lines.append("")

    POT_PATH.parent.mkdir(parents=True, exist_ok=True)
    POT_PATH.write_text("\n".join(lines), encoding="utf-8")


def main() -> None:
    messages: OrderedDict[str, list[str]] = OrderedDict()
    for path in sorted(SRC_DIR.rglob("*.rs")):
        rel_path = path.relative_to(ROOT)
        for line_no, message in extract_messages(path):
            ref = f"{rel_path}:{line_no}"
            messages.setdefault(message, []).append(ref)

    write_pot(messages)
    print(f"wrote {POT_PATH.relative_to(ROOT)} with {len(messages)} messages")


if __name__ == "__main__":
    main()
