# Asset sources and version records

## Authoritative starting points

- [Gacha Setup repository](https://github.com/PaoloESAN/gacha-setup): implementation,
  release history and bundled dependencies. Prefer a tag/commit over moving main.
- [Releases](https://github.com/PaoloESAN/gacha-setup/releases): determine current
  availability live; documentation changelogs can lag releases.
- [Quickstart](https://gacha-setup.pages.dev/quickstart/): supported Blender,
  installation and basic setup. The 2026-09-14 page specifies Blender 5.2+.
- [Character Assets](https://gacha-setup.pages.dev/character-models/): links to
  HoyoToon and hiragara resources. The plugin itself does not supply game models.
- [HoyoToon Genshin characters](https://assets.hoyotoon.com/?game=gi&type=characters):
  character/variant selection and file downloads.

Record where a file was retrieved, not merely a page with a matching character.
A hash match establishes byte identity; it does not prove which site a previous
user originally visited. Keep channel, variant and version separate from display
name. A catalogue character listing is not proof that the model files are available.

## HoyoToon retrieval

Use the site's visible character and variant selectors when needed. Prefer the
site's public data requests for catalogue/file inspection when available. As
observed 2026-09-14, the frontend requests:

```text
GET https://assets.hoyotoon.com/api/catalog?game=gi&type=characters
GET https://assets.hoyotoon.com/api/remote-assets?game=gi&type=characters&ids=CHARACTER_ID
```

These are observed web-application endpoints, not a promised stable public SDK.
Re-discover them if responses change; do not guess endpoints, brute-force IDs or
substitute another release/channel silently. The catalogue's `snapshot.page.items`
contains `characterId`, `displayName`, `primaryArtName`, and metadata. Follow
pagination if `isDone` is false. Remote `files` contain relative `path`, `size`,
`etag`, `channel` and `downloadUrl`. Use returned URLs, preserve Materials/Textures
layout, and verify received size/content. Treat etag as opaque, not as SHA-256.
If an active release/revision is supplied, pin it across related requests.
No activeRelease was returned for the verified Odette file listing.

A selected variant can require loose files and shared textures; do not download
only the FBX. Avoid mixing similarly named files across variants. Keep raw
responses and downloaded archives/files in a local evidence/cache directory,
not in the distributable skill. Check any source-specific redistribution terms
before publishing assets; acquisition and permission to redistribute are separate.

## Source manifest

Record game, ID/internal name, variant/channel, listing and file URLs, retrieval
UTC time, archive/file SHA-256, size, source directory and any transformations.
Use `scripts/source_manifest.py` to inventory the exact isolated input. A source
with multiple FBXs is ambiguous for automatic folder import. Keep Mixamo clips
outside that input. For source comparison, retain both byte hashes and optional
parsed-JSON equality results.

## Plugin upgrades

1. Check the real release tag and minimum Blender from the downloaded package's
   `setup_wizard/__init__.py` (`bl_info`). Do not trust cached “Latest” labels alone.
2. Back up the installed module and `userpref.blend`; keep the previous archive.
3. Install the release asset, enable `setup_wizard`, and verify in a fresh Blender
   process that its file path, `bl_info['version']` and enabled state are correct.
4. Save preferences when installation is requested, but do not overwrite authored
   scenes. Installing a plugin and upgrading an existing scene are distinct actions.
5. Compare shader artifact hashes and import behavior. Keep project reference locks
   unchanged until the corresponding upgrade qualification has passed.

Snapshot: v3.6.4 was installed and fresh-start verified on Blender 5.2.1 LTS.
Archive SHA-256 `7042aeeaac488ac60515773a1414bd643175f91c950bba2a05b2e6d826744321`.
This is historical evidence, not an instruction to install that version forever.
