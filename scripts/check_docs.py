#!/usr/bin/env python3
"""Check local link targets in project-owned Markdown documents."""

import re
from pathlib import Path
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).resolve().parents[1]
FILES = [ROOT / name for name in ("README.md", "AGENTS.md", "ARCHITECTURE.md")]
FILES.extend(sorted((ROOT / "docs").rglob("*.md")))
FILES.extend(sorted((ROOT / "design").rglob("*.md")))
LINK = re.compile(r"\[[^\]]*\]\(([^\s)]+)(?:\s+\"[^\"]*\")?\)")


def main():
    errors = []
    checked = 0
    for source in FILES:
        if not source.is_file():
            errors.append(f"Missing document: {source.relative_to(ROOT)}")
            continue
        for match in LINK.finditer(source.read_text()):
            href = urlsplit(match.group(1).strip("<>"))
            if href.scheme or href.netloc or not href.path:
                continue
            target = source.parent / unquote(href.path)
            checked += 1
            if not target.exists():
                errors.append(f"{source.relative_to(ROOT)}: missing {href.path}")
    if errors:
        print("\n".join(errors))
        raise SystemExit(1)
    print(f"OK: {checked} local link targets across {len(FILES)} project documents.")
    print("External URLs, fragment anchors, and vendored skill documents are not checked.")


if __name__ == "__main__":
    main()
