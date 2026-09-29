# Agent entrypoint

Read README.md, CONTRIBUTING.md and the relevant architecture/authoring/reference
contract before changing that domain. The project is independent: no sibling game
checkout, game exporter, processed game model or personal absolute path may become
an operational requirement. Current source attribution is maintained in THIRD_PARTY_NOTICES.md.

Continue authorized reversible maintenance and local validation without asking
for repeated stage approval. Keep current assets and unrelated working changes.
Do not create commits or publish unless requested. Only the current version is
supported; retired compatibility APIs and obsolete historical material are removed.

Use the user-level agent skills when available:

- `~/.agents/skills/blender-gacha/SKILL.md`: source acquisition and Blender authoring.
- `~/.agents/skills/bevy-stylized/SKILL.md`: export, runtime mapping and validation.
- `~/.agents/skills/bevy-copilot/SKILL.md`: optional live BRP inspection when exposed.

These skills are maintained at the user level (`~/.agents/skills/`) and are no
longer part of the repository; personal skill copies must not override current
project paths or contracts. Every task remains executable
through documented tools without an AI client, Blender MCP or a personal skill.

Validation is local-only. Do not enable GitHub Actions or launch remote CI.

Use `tools/dev.py` for standard tasks. Serialize Cargo builds and GPU/performance
runs. Required GPU/asset/baseline absence is BLOCKED, not PASS. Keep engineering
independence, image regression and upstream parity as distinct reported outcomes.

Authoring acceptance uses the qualified version/hash in
`assets/sources/gacha-authoring.json` and the scope of the active recipe. There
is no supported old-version lane. Do not restore compatibility code or rewrite
dated measurement outcomes. Keep proposed ADRs distinct from accepted decisions.
Documentation-only changes use document/reference checks without Cargo/GPU runs.
