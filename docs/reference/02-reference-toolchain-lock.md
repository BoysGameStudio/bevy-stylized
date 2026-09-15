# Reference Toolchain Lock

> Current applicability (2026-09-15): the current authoring target is pinned in
> [gacha-authoring.json](../../assets/sources/gacha-authoring.json). Historical v3.6.2
> examples or continuation requirements below are opt-in evidence, not current gates.
> [Current static qualification](../quality/realtime-validation-2026-09-15.json)
> records the later scope; original dated outcomes below are preserved.

> **Status:** Normative architecture specification
> **Authority:** This document is authoritative for its domain; the archived Master Plan is historical only.


Golden Reference 必须绑定**明确版本**。

禁止：

```text
Blender = latest
Gacha Setup = latest
```

作为 Golden 的永久定义。

每一套 Reference Baseline 都必须保存：

```text
Blender exact version
Gacha Setup release
Gacha Setup tag
Gacha Setup commit SHA
Release package SHA-256
Reference Family ID
Main Shader Artifact path + digest
Outline Artifact path + digest
Post Artifact path + digest
Reference Render Engine / Color Management / View Transform
Canonical Reference OS / GPU backend / GPU+driver (when renderer-dependent)
Export Contract version
bevy-stylized Genshin Reference Profile schema version
Reference fixture revision
Camera/Lighting preset revision
Golden revision
```

建议建立：

```text
docs/reference/02-reference-toolchain-lock.md
```

以及机器可读：

```text
reference-toolchain.toml
```

示意结构：

```toml
[blender]
series = "5.2 LTS"
version = "<exact validated Blender version>"

[gacha_setup]
repository = "PaoloESAN/gacha-setup"
release = "v3.6.2"
tag = "v3.6.2"
commit = "069e51f3fd89cdc66b4c2741798ede11a5fb16b5"
release_asset = "Gacha_Setup-v3.6.2.zip"
sha256 = "f5ab8206464130962234a7ccab19c7d212a048c848ce3e69518f84fc41a39999"

[export]
contract = "blender-gltf-npr-v1"

[reference_family]
id = "genshin-festivity-gacha-setup-v1"

[reference_family.artifacts]
main = "setup_wizard/shaders/gi/HoYoverse - Genshin Impact v3.4.blend"
main_git_blob = "7637f1d18a63188ca08870fb554d949dafefaecf"
outline = "setup_wizard/shaders/gi/HoYoverse - Genshin Impact Outlines v3.blend"
outline_git_blob = "532e0860065acea1e427f1d4033f69626793036b"
post = "setup_wizard/shaders/gi/HoYoverse - Genshin Impact Post-Processing.blend"
post_git_blob = "7cafd3d5fcb77e5e7efa8037531e085887f57d5e"

[profile]
genshin_schema = "v1"
```

上面的 Gacha Setup `v3.6.2` 是 **2026-09-11 文档修订时的最新正式 Release 和候选 Reference Upstream**，不是“永远自动使用最新版本”的规则。真正进入 Golden Baseline 前仍必须完成完整 Reference Validation。

若将来最新 release 变成 v3.7/v4：

```text
New Upstream Release
        ↓
Candidate Upgrade Branch
        ↓
Reference Import + Export
        ↓
Golden Compare
        ↓
Semantic Diff Review
        ↓
Accept / Reject
```

不能让 CI 自动跟随最新 release。

---
