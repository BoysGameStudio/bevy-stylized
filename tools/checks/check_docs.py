#!/usr/bin/env python3
from pathlib import Path
import re, sys, os, subprocess
from doc_commands import missing_entrypoints

ROOT = Path(__file__).resolve().parents[2]
def project_markdown_files():
    # Include untracked project documents, never generated caches or downloaded assets.
    if (ROOT / ".git").exists():
        result = subprocess.run(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
                                cwd=ROOT, capture_output=True, check=True)
        return sorted({ROOT / os.fsdecode(name) for name in result.stdout.split(b"\0")
                       if name.endswith(b".md") and (ROOT / os.fsdecode(name)).is_file()
                       and not (ROOT / os.fsdecode(name)).is_symlink()})
    result = []
    for directory, dirs, files in os.walk(ROOT):
        dirs[:] = [d for d in dirs if d not in {".git", "target", "local", "golden-local", "__pycache__"}
                   and not (Path(directory) / d).is_symlink()]
        result.extend(Path(directory) / name for name in files if name.endswith(".md")
                      and not (Path(directory) / name).is_symlink())
    return sorted(result)

MARKDOWN_FILES = project_markdown_files()
ERRORS = []
WARNINGS = []

def err(msg): ERRORS.append(msg)
def warn(msg): WARNINGS.append(msg)

def text(path): return path.read_text(encoding="utf-8")

# 1. Every non-archive Markdown file should have an H1.
for f in MARKDOWN_FILES:
    lines = text(f).splitlines()
    if not any(line.startswith("# ") for line in lines):
        err(f"{f.relative_to(ROOT)}: missing H1")

# 2. Code fences must be balanced.
for f in MARKDOWN_FILES:
    if text(f).count("```") % 2:
        err(f"{f.relative_to(ROOT)}: unbalanced code fences")

# Literal Python commands also occur in shell/text fences without Markdown links.
for f in MARKDOWN_FILES:
    for command in missing_entrypoints(text(f), ROOT, f):
        err(f"{f.relative_to(ROOT)}: missing Python command entrypoint: {command}")

# 3. ADR integrity.
adr_dir = ROOT / "docs/adr"
adr_files = sorted(adr_dir.glob("[0-9][0-9][0-9][0-9]-*.md"))
ids = []
valid_status = {"Proposed","V1 Blocker","Accepted","Rejected","Superseded"}
for f in adr_files:
    t = text(f)
    fid = f.name[:4]
    ids.append(fid)
    h = re.search(r"^# ADR-(\d{4}):", t, re.M)
    if not h or h.group(1) != fid:
        err(f"{f.relative_to(ROOT)}: ADR H1 id does not match filename")
    sm = re.search(r"- \*\*Status:\*\* (.+)", t)
    status = sm.group(1).strip() if sm else None
    if status not in valid_status:
        err(f"{f.relative_to(ROOT)}: invalid/missing ADR status {status!r}")
    if status == "Accepted":
        for section in ("Decision","Evidence","Consequences"):
            m = re.search(rf"^## {section}\s*$([\s\S]*?)(?=^## |\Z)", t, re.M)
            if not m or not m.group(1).strip() or re.search(r"\bTBD\b", m.group(1), re.I):
                err(f"{f.relative_to(ROOT)}: Accepted ADR has incomplete {section}")
if len(ids) != len(set(ids)):
    err("duplicate ADR ids")

# 4. Traceability must reference every ADR, especially every V1 Blocker.
trace = text(ROOT/"TRACEABILITY.md")
trace_ids = set(re.findall(r"ADR-(\d{4})", trace))
all_ids = set(ids)
for missing in sorted(all_ids - trace_ids):
    err(f"TRACEABILITY.md: missing ADR-{missing}")

# 5. ADR index must match real files.
index = text(adr_dir/"ADR_INDEX.md")
index_ids = set(re.findall(r"ADR-(\d{4})", index))
if index_ids != all_ids:
    err(f"ADR_INDEX mismatch: missing={sorted(all_ids-index_ids)} extra={sorted(index_ids-all_ids)}")

# 6. Local Markdown links to .md files must resolve.
link_re = re.compile(r"\[[^\]]*\]\(([^)]+\.md(?:#[^)]+)?)\)")
for f in MARKDOWN_FILES:
    for target in link_re.findall(text(f)):
        target_path = target.split("#",1)[0]
        if target_path.startswith(("http://","https://")):
            continue
        resolved = (f.parent / target_path).resolve()
        try:
            resolved.relative_to(ROOT.resolve())
        except ValueError:
            err(f"{f.relative_to(ROOT)}: link escapes documentation root: {target}")
            continue
        if not resolved.exists():
            err(f"{f.relative_to(ROOT)}: broken Markdown link: {target}")

# Current navigation also contains source/schema links. Bound this check to
# maintained entrypoints, not historical examples or arbitrary Markdown snippets.
current_navigation = [
    ROOT/'README.md', ROOT/'ARCHITECTURE.md',
]
for f in current_navigation:
    for target in re.findall(r"\[[^\]]*\]\(([^)]+)\)", text(f)):
        if target.startswith(("https://", "http://", "#")):
            continue
        path = (f.parent / target.split("#", 1)[0]).resolve()
        if not path.is_relative_to(ROOT) or not path.exists():
            err(f"{f.relative_to(ROOT)}: invalid current contract/source link: {target}")

# The README's qualified version is an explicit claim, not an independent lock.
import json
source = json.loads(text(ROOT/'assets/sources/gacha-authoring.json'))
claimed = re.search(r"currently qualified\s+(v[0-9.]+)", text(ROOT/'README.md'))
if not claimed or claimed.group(1) != source['release']:
    err('README current authoring qualification differs from source lock')

# 7. Root-style textual paths like docs/foo.md should exist.
path_re = re.compile(r"(?<![\w/])((?:docs|tools)/[A-Za-z0-9_./-]+\.md)")
for f in MARKDOWN_FILES:
    for p in path_re.findall(text(f)):
        if not (ROOT/p).exists():
            err(f"{f.relative_to(ROOT)}: stale documentation path: {p}")

# 9. Forbidden legacy terminology outside archive.
forbidden = [
    "Stable Profile ABI",
    "bevy-stylized-profile-gacha",
    "NprQualityTier",
    "StylizedMaterialClass",
]
for f in MARKDOWN_FILES:
    t = text(f)
    for term in forbidden:
        if term in t:
            err(f"{f.relative_to(ROOT)}: forbidden legacy term: {term}")

# 10. Required governance files.
required = [
    "ARCHITECTURE.md","GLOSSARY.md","TRACEABILITY.md",
    "THIRD_PARTY_NOTICES.md","docs/adr/ADR_INDEX.md",
    "docs/quality/semantic-contract.md","docs/performance/targets.md",
    "docs/platforms/SUPPORT_MATRIX.md",
]
for p in required:
    if not (ROOT/p).exists():
        err(f"missing required file: {p}")

for w in WARNINGS:
    print("WARN:", w)
for e in ERRORS:
    print("ERROR:", e)
print(f"Checked {len(MARKDOWN_FILES)} Markdown files and {len(adr_files)} ADRs.")
if ERRORS:
    print(f"FAILED with {len(ERRORS)} error(s).")
    sys.exit(1)
print("PASS")
