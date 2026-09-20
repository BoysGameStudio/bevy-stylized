"""Validate literal local Python entrypoints in documented commands."""
from pathlib import Path
import re

COMMAND = re.compile(
    r'''(?<![\w/.-])(?:python(?:3(?:\.\d+)?)?|--python)\s+'''
    r'''(?:(?:-u|-B)\s+)*(?:"([^"\n]+\.py)"|'([^'\n]+\.py)'|([^\s`"'<>]+\.py))'''
    r'''(?=\s|$|[;`])'''
)


def missing_entrypoints(content: str, root: Path, document: Path) -> list[str]:
    root = root.resolve()
    # Examples may run from the project or from their documented skill/tool folder.
    bases = [root, *(p for p in document.resolve().parents if p.is_relative_to(root))]
    missing = set()
    for match in COMMAND.finditer(content):
        name = next(part for part in match.groups() if part is not None)
        path = Path(name)
        if path.is_absolute() or any(marker in name for marker in ('$', '{', '}', '<', '>')):
            continue  # Supplied external paths/placeholders are not repository entrypoints.
        if not any((base / path).is_file() for base in bases):
            missing.add(name)
    return sorted(missing)
