# Agent entrypoint

Read README.md, CONTRIBUTING.md and the relevant architecture/authoring/reference
contract before changing that domain. The project is independent: no sibling game
checkout, game exporter, processed game model or personal absolute path may become
an operational requirement. Current source attribution is maintained in THIRD_PARTY_NOTICES.md.

Continue authorized reversible maintenance and local validation without asking
for repeated stage approval. Keep current assets and unrelated working changes.
Do not create commits or publish unless requested. Only the current version is
supported; retired compatibility APIs and obsolete historical material are removed.

Use repository skills when applicable:

- `.agents/skills/blender-gacha/SKILL.md`: source acquisition and Blender authoring.
- `.agents/skills/bevy-stylized/SKILL.md`: export, runtime mapping and validation.
- `.agents/skills/bevy-copilot/SKILL.md`: optional live BRP inspection when exposed.

The repository is the maintained source of these workflows. Personal skill copies
must not override current project paths or contracts. Every task remains executable
through documented tools without an AI client, Blender MCP or a personal skill.

Validation is local-only. Do not enable GitHub Actions or launch remote CI.
Historical workflow definitions were removed; Git history retains their provenance.

Use `tools/dev.py` for standard tasks. Serialize Cargo builds and GPU/performance
runs. Required GPU/asset/baseline absence is BLOCKED, not PASS. Keep engineering
independence, image regression and upstream parity as distinct reported outcomes.

Gacha Setup acceptance targets only the latest stable release, pinned by exact
version/hash in `assets/sources/gacha-authoring.json` after qualification. As of
2026-09-15 this is v3.6.4. Older release compatibility is not a project requirement.
The v3.6.2 reference lane and its recorded failures are historical, opt-in evidence,
not current blockers or mandatory isolation inputs. Do not restore old-version
support obligations without a user request. Preserve dated evidence without
rewriting its historical outcomes.
