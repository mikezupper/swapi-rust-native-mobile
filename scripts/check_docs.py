#!/usr/bin/env python3
"""Check local files and heading anchors in project-owned documentation."""

import re
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).resolve().parents[1]
FILES = [ROOT / name for name in ("README.md", "AGENTS.md", "ARCHITECTURE.md")]
FILES.extend(sorted((ROOT / "docs").rglob("*.md")))
FILES.extend(sorted((ROOT / "design").rglob("*.md")))
LINK = re.compile(r"\[[^\]]*\]\(([^\s)]+)(?:\s+\"[^\"]*\")?\)")


class HTMLReferences(HTMLParser):
    def __init__(self):
        super().__init__()
        self.links = []
        self.anchors = set()

    def handle_starttag(self, tag, attributes):
        attributes = dict(attributes)
        if attributes.get('id'):
            self.anchors.add(attributes['id'])
        attribute = 'href' if tag == 'a' else 'src' if tag in ('img', 'video', 'source') else None
        if attribute and attributes.get(attribute):
            self.links.append(attributes[attribute])


def prose(text):
    """Exclude fenced examples from link and heading discovery."""
    lines = []
    fence = None
    for line in text.splitlines():
        marker = re.match(r'^\s{0,3}(`{3,}|~{3,})', line)
        if marker:
            value = marker[1]
            if fence is None:
                fence = value
            elif value[0] == fence[0] and len(value) >= len(fence):
                fence = None
        elif fence is None:
            lines.append(line)
    return '\n'.join(lines)


def anchors(text):
    html = HTMLReferences()
    html.feed(prose(text))
    found = html.anchors
    for heading in re.findall(r'^\s{0,3}#{1,6}\s+(.+?)\s*#*$', prose(text), re.MULTILINE):
        heading = re.sub(r'\[([^\]]+)\]\([^)]*\)', r'\1', heading)
        heading = re.sub(r'<[^>]*>', '', heading)
        slug = re.sub(r'[^\w -]', '', heading.lower()).replace(' ', '-')
        candidate = slug
        suffix = 0
        while candidate in found:
            suffix += 1
            candidate = f'{slug}-{suffix}'
        found.add(candidate)
    return found


def main():
    errors = []
    checked = 0
    for source in FILES:
        if not source.is_file():
            errors.append(f"Missing document: {source.relative_to(ROOT)}")
            continue
        text = prose(source.read_text())
        html = HTMLReferences()
        html.feed(text)
        links = [match.group(1).strip('<>') for match in LINK.finditer(text)] + html.links
        for link in links:
            href = urlsplit(link)
            if href.scheme or href.netloc:
                continue
            target = source.parent / unquote(href.path) if href.path else source
            checked += 1
            if not target.exists():
                errors.append(f"{source.relative_to(ROOT)}: missing {href.path}")
            elif href.fragment and target.suffix in ('.md', '.html') and unquote(href.fragment) not in anchors(target.read_text()):
                errors.append(f"{source.relative_to(ROOT)}: missing {href.path}#{href.fragment}")
    if errors:
        print("\n".join(errors))
        raise SystemExit(1)
    print(f"OK: {checked} local link targets across {len(FILES)} project documents.")
    print("Local heading/HTML anchors and HTML image links are checked; external URLs and vendored skills are not.")


if __name__ == "__main__":
    main()
