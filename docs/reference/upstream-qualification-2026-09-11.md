# Upstream 资格评估记录 2026-09-11

> **Status:** Working record, non-normative
> **关联：** docs/roadmap/02-iteration-plan.md It-1a 交付物 1（上游资格评估）；候选值来源 docs/reference/02-reference-toolchain-lock.md；机器可读结果 docs/reference/reference-toolchain.toml
>
> **⚠ 注意：** §1–§6 基于**错误的上游仓库**（michael-gh1 Setup Wizard addon，并非 Gacha Setup 本体），其"不存在"结论已被推翻；以 **§7 修正（同日）** 为最终结论。旧节保留作为审计轨迹。

## 1. 评估对象与候选值

候选 Reference Upstream：**Gacha Setup v3.6.2**，即 GitHub 仓库 `michael-gh1/Addons-And-Tools-For-Blender-miHoYo-Shaders` 的 release 包（捆绑 festivities/Blender-miHoYo-Shaders 谱系 miHoYo 风格 shader 的 Blender 设置向导 addon）。docs/reference/02-reference-toolchain-lock.md 记录的候选值：

- release/tag：`v3.6.2`
- commit：`069e51f3fd89cdc66b4c2741798ede11a5fb16b5`
- release asset：`Gacha_Setup-v3.6.2.zip`
- sha256：`f5ab8206464130962234a7ccab19c7d212a048c848ce3e69518f84fc41a39999`
- 预期包内 artifacts（附 git blob SHA）：
  - `setup_wizard/shaders/gi/HoYoverse - Genshin Impact v3.4.blend`（`7637f1d1…`）
  - `setup_wizard/shaders/gi/HoYoverse - Genshin Impact Outlines v3.blend`（`532e0860…`）
  - `setup_wizard/shaders/gi/HoYoverse - Genshin Impact Post-Processing.blend`（`7cafd3d5…`）

## 2. 检查方法与来源

GitHub REST API（api.github.com）当日匿名配额已耗尽（HTTP 403 rate limit，reset 16:11 CST），且本机 `gh` token 失效，故主要证据来自**不需要 API 配额**的等价权威来源：

| 检查项 | 命令 / URL |
|---|---|
| 全量 tag 列表 | `git ls-remote --tags https://github.com/michael-gh1/Addons-And-Tools-For-Blender-miHoYo-Shaders.git` |
| release 列表与 release notes | `https://github.com/michael-gh1/Addons-And-Tools-For-Blender-miHoYo-Shaders/releases`（HTML）与 `releases.atom`（Atom feed） |
| v3.6.2 tag 页面 | `https://github.com/michael-gh1/Addons-And-Tools-For-Blender-miHoYo-Shaders/releases/tag/v3.6.2`（curl -L） |
| v3.6.2 asset 下载 | `https://github.com/michael-gh1/Addons-And-Tools-For-Blender-miHoYo-Shaders/releases/download/v3.6.2/Gacha_Setup-v3.6.2.zip`（curl -L） |
| 候选 commit 存在性 | `…/commit/069e51f3…`（HTML）+ blobless clone 内 `git cat-file -t` |
| 记录 blob SHA 存在性 | `git clone --filter=blob:none` 后 `git rev-list --objects --all`（对 michael-gh1 与 festivize 两仓库） |
| 实际 release asset | `…/releases/expanded_assets/v2.7.4`（HTML）→ curl -L 下载 |
| 包内容 | `unzip -l` / `unzip -o` 至 `target/tmp/gacha-setup/`（gitignored） |
| artifact digest | `git hash-object <file>` |
| LICENSE | `https://raw.githubusercontent.com/michael-gh1/Addons-And-Tools-For-Blender-miHoYo-Shaders/main/LICENSE`；`codeload.github.com/festivize/Blender-miHoYo-Shaders/zip/refs/heads/main`；zip 内 LICENSE 文件 |
| shader 谱系仓库 | `git ls-remote https://github.com/festivities/Blender-miHoYo-Shaders.git`（HEAD 与 festivize 相同，`festivities` 重定向至 `festivize`） |

API 配额恢复后（当日 16:13 CST）已用官方 REST API 复核，结论一致：

- `GET /repos/michael-gh1/Addons-And-Tools-For-Blender-miHoYo-Shaders/releases?per_page=5`：最新 release 依次为 v2.7.4（published 2026-08-05T03:43:34Z，assets `Character_Setup_Wizard_Addon_v2.7.4.zip` 1727885 B / `..._Standalone.zip` 56561437 B）、v2.7.3、v2.7.2、v2.7.1、v2.7.0，无 v3.x；
- `GET .../releases/tags/v3.6.2` → HTTP 404；`GET .../git/refs/tags/v3.6.2` → HTTP 404。

## 3. 逐项验证结果

| 候选值 | 结果 | 证据 |
|---|---|---|
| release v3.6.2 存在 | **FAIL（不存在）** | 上游全部 tag 止于 `v2.7.4`（v0.9.0→v2.7.4，无任何 v3.x）；releases 列表/Atom feed 最新 release 为 v2.7.4（published 2026-08-05T03:43:34Z，"v2.7.4 \| Character Setup Wizard"）；`releases/tag/v3.6.2` 返回 HTTP 404 |
| commit `069e51f3…` | **FAIL（不存在）** | commit 页面 HTTP 404；blobless clone 中 `git cat-file -t` 报 `upload-pack: not our ref` |
| asset `Gacha_Setup-v3.6.2.zip` | **FAIL（不存在）** | 下载 URL HTTP 404。v2.7.4 实际 asset 为 `Character_Setup_Wizard_Addon_v2.7.4.zip`（1.73 MB，sha256 `e42ddc229548f635ffb11df2876ae5a2825fc2f79e68c825084a9d4c23b404f3`）与 `Character_Setup_Wizard_Addon_v2.7.4_Standalone.zip`（56.6 MB，sha256 `d34c21f832fae1a092c0367f3cb3f7d7d6bae5d2b86ad63c4ba0b2df6b00d700`）。资产命名已从 "Gacha Setup" 系改为 "Character Setup Wizard" 系 |
| sha256 `f5ab8206…` | **FAIL（无法对应任何 asset）** | 目标 asset 不存在；最接近的实际 asset digest 见上，均不匹配 |
| 包内路径 `setup_wizard/shaders/gi/*.blend` | **FAIL（不存在）** | v2.7.4 两个 zip 均无 `shaders/` 目录；michael-gh1 仓库**全部历史**中也不存在 `shaders/gi` 或任何 `HoYoverse` 路径。v2.7.4 起 shader 不随 addon 分发，README 指示用户自行从 shader 谱系仓库获取 |
| artifact 文件名本身 | **部分成立（位置不同）** | 三个文件名存在于 `festivize/Blender-miHoYo-Shaders`（festivities 重定向后）仓库**根目录**，HEAD `b79f0f3fb47b4de2b6d187ee6b66d93cb965968d`（2024-05-24 "feat: v3.4 - Lighting Panel"） |
| blob `532e0860…`（Outlines v3） | **PASS** | `git hash-object` 实测一致；且存在于 festivize 仓库历史 |
| blob `7cafd3d5…`（Post-Processing） | **PASS** | 同上 |
| blob `7637f1d1…`（main v3.4） | **FAIL（不匹配）** | festivize HEAD 实测为 `1ca1912e0962ba131164868c6a46836f86009f05`；记录值 `7637f1d1…` 在 festivize **全部历史**及 michael-gh1 全部历史中均不存在 |
| LICENSE（预期 GPL-3.0） | **PASS** | michael-gh1 仓库根 `LICENSE`（main 分支，35148 字节）为 GNU GPL v3（"Version 3, 29 June 2007"）；festivize 仓库根 `LICENSE`（35149 字节）同为 GPL-3.0；v2.7.4 zip 内 `setup_wizard/dependencies/LICENSE` 与 `setup_wizard/addon_updater/LICENSE.txt` 亦为 GPL-3.0 文本 |
| Blender 版本兼容性（本机 5.2.1） | **PENDING（未证实）** | addon `__init__.py` bl_info `"blender": (3, 3, 0)`（最低版本）；README："tested on **Blender (and Goo Blender) Version >= 3.3.0**"，未声明上限，未提及 5.x；v2.7.4 release notes 未涉及 Blender 版本。5.2.1 兼容性须实测（addon 注册/导入/材质与描边搭建） |

## 4. 差异汇总（ discrepancies ）

1. **候选版本整体不存在**：v3.6.2 的 release/tag/commit/asset/digest 五项候选值无一在上游成立；记录中的候选值疑似来源有误（或取自他处/未来版本），须按治理流程修正 docs/reference/02-reference-toolchain-lock.md 的候选记录。
2. **命名漂移**：上游 addon 现名 "HoYoverse Setup Wizard"（bl_info），release 资产名 "Character_Setup_Wizard_Addon_*"，无 "Gacha Setup" 命名的现行资产。
3. **artifact 归属错位**：三个 shader blend 属于 shader 谱系仓库（festivize/Blender-miHoYo-Shaders）根目录，而非 setup-wizard release 包内 `setup_wizard/shaders/gi/`。Reference Family 实际由**两个上游仓库**构成（addon + shader），锁定记录需分别固定。
4. **main shader blob 不匹配**：记录 blob 不存在于任何可查历史；谱系仓库 HEAD 的 main shader 为另一 blob。
5. **festivities 组织重定向**：`github.com/festivities/Blender-miHoYo-Shaders` 重定向至 `festivize`，provenance 记录应写重定向后的规范 URL。

## 5. 结论

**v3.6.2 + 本机 Blender 5.2.1 不能作为 Reference Baseline 候选** —— v3.6.2 在上游不存在，无可锁定对象。

可行的替代路径（须维护者决策，写入 ADR-0011 证据）：

1. **替代候选**：`Character Setup Wizard v2.7.4`（上游现行最新正式 release，commit `134120e9ab04e6058472495d60099d5ec1eaa9ad`，2026-08-05）+ `festivize/Blender-miHoYo-Shaders @ b79f0f3`（双仓库 Reference Family）。两者 LICENSE 均为 GPL-3.0，与 ADR-0017 的既有假设一致。
2. **Blender 版本**：addon 仅声明 ≥3.3.0 且未验证 5.x。It-1a 风险表预案仍有效：先以本机 snap Blender 5.2.1 实测 v2.7.4 addon；不满足则安装被锁的旧版 Blender（与 snap 并存），被锁版本以实测通过的最低官方支持版本为准。
3. **本地存档**：v2.7.4 两个 asset 与 festivize HEAD 源码 zip 已存于 `target/tmp/gacha-setup/`（gitignored），可作为"上游不可得时降级方案"的存档种子；正式存档与合规确认（GPL-3.0 派生物不入库，遵守 ADR-0017）待维护者决策后执行。

## 6. 对 It-1a 后续步骤的阻塞

- **阻塞交付物 2（Toolchain Lock 定稿）**：无已验证的 addon release 可锁，reference-toolchain.toml 的 `[gacha_setup]` 全部 PENDING；需维护者对替代候选（v2.7.4 + festivize HEAD）拍板。
- **阻塞 ADR-0011 草案**：候选值修正须先走文档治理流程修订 docs/reference/02-reference-toolchain-lock.md。
- **Blender 兼容性实测**（交付物 2/3 的前置）：需在本机 5.2.1 上实际安装并运行 v2.7.4 addon，确认注册、材质导入、几何节点描边在 5.x 下可用；不通过则触发"安装被锁版本"预案。
- 不阻塞：main shader blob 的 PENDING 不阻碍导出契约（交付物 3）与导出桥（交付物 4）先行设计。

## 7. 修正（同日）：改用正确上游 PaoloESAN/gacha-setup 重验

§1–§6 误将 `michael-gh1/Addons-And-Tools-For-Blender-miHoYo-Shaders`（Setup Wizard addon 本体）当作 Gacha Setup 上游。真正的 Gacha Setup 上游为 **`PaoloESAN/gacha-setup`** —— 按包内 `setup_wizard/README.md` 的 "License & Attribution" 节，它是 michael-gh1 仓库的 fork（"derivative work modified in 2026"），两者确为不同仓库。以下用正确上游重跑全部检查项（API 当日再次限流，证据同样来自 git 协议 / Atom / HTML / 包内容，均为决定性来源）：

| 检查项 | 命令 / URL |
|---|---|
| tag → commit | `git ls-remote --tags https://github.com/PaoloESAN/gacha-setup.git` |
| release 列表与 notes | `https://github.com/PaoloESAN/gacha-setup/releases.atom` |
| asset 列表 | `https://github.com/PaoloESAN/gacha-setup/releases/expanded_assets/v3.6.2` |
| asset 下载 | `https://github.com/PaoloESAN/gacha-setup/releases/download/v3.6.2/Gacha_Setup-v3.6.2.zip` → `target/tmp/gacha-setup/Gacha_Setup-v3.6.2.zip` |
| 包内容 / digest | `unzip -o` 至 `target/tmp/gacha-setup/extracted/`；`sha256sum`；`git hash-object` |
| LICENSE | `https://raw.githubusercontent.com/PaoloESAN/gacha-setup/main/LICENSE`；包内 README / `setup_wizard/addon_updater/LICENSE.txt` |

### 逐项验证结果（正确上游）

| 候选值 | 结果 | 证据 |
|---|---|---|
| release v3.6.2 存在 | **PASS** | tag 列表含 v3.0.0–v3.6.2；Atom feed 最新 entry 为 v3.6.2，published **2026-09-09T14:29:15Z**，标题 "Update v3.6.2"（ZZZ 材质融合修复）；`expanded_assets/v3.6.2` 列出唯一 asset |
| commit `069e51f3fd89cdc66b4c2741798ede11a5fb16b5` | **PASS** | `git ls-remote`：`refs/tags/v3.6.2` 解析到该 SHA，与记录值逐字符一致 |
| asset `Gacha_Setup-v3.6.2.zip` | **PASS** | 资产名完全一致，18.9 MB（19,792,155 字节），HTTP 200 |
| sha256 `f5ab8206…41a39999` | **PASS** | 本地下载实测 sha256 与记录值逐字符一致 |
| 路径 `setup_wizard/shaders/gi/HoYoverse - Genshin Impact v3.4.blend` | **PASS** | 包内存在 |
| 路径 `…/HoYoverse - Genshin Impact Outlines v3.blend` | **PASS** | 包内存在 |
| 路径 `…/HoYoverse - Genshin Impact Post-Processing.blend` | **PASS** | 包内存在 |
| blob `7637f1d1…`（main v3.4） | **PASS** | `git hash-object` 实测一致（注意：与 festivize 谱系仓库 HEAD 的同名文件 blob `1ca1912e…` 不同，即包内为 vendor 后改动的副本） |
| blob `532e0860…`（Outlines v3） | **PASS** | 实测一致（与谱系仓库 HEAD 相同） |
| blob `7cafd3d5…`（Post-Processing） | **PASS** | 实测一致（与谱系仓库 HEAD 相同） |
| LICENSE（预期 GPL-3.0） | **PASS** | 仓库根 `LICENSE`（main，35148 字节）为 GPL-3.0 原文；包内 `setup_wizard/README.md` "License & Attribution" 节声明 GPL v3 衍生作品；`setup_wizard/shaders/gi/README.md` License 节声明 GPL-3.0 并注明原始仓库 festivize/Blender-miHoYo-Shaders；bl_info `"license": "GPL-3.0-or-later"`；`addon_updater/LICENSE.txt` 为 GPL-3.0 原文 |
| Blender 版本兼容性（本机 5.2.1） | **PASS（声明层面）** | bl_info `"blender": (5, 2, 0)`（最低版本），description 明确 "in Blender 5.2+"；README："designed specifically for official **Blender 5.2+**"，旧版本与 Goo Blender "NOT recommended"。本机 snap Blender 5.2.1 LTS ≥ 5.2.0，满足声明要求；release notes 未另述版本要求。Golden 捕获前的 addon 实测（注册/材质/描边搭建）仍是 It-1a 后续动作 |

### 结论（更新）

**v3.6.2 + 本机 Blender 5.2.1 可以作为 Reference Baseline 候选**：全部十项候选值（release/tag/commit/asset/sha256/三路径/三 blob）在正确上游逐一验证通过，license 为 GPL-3.0（-or-later），上游声明的 Blender 5.2+ 要求与本机 5.2.1 匹配，**无需降级或安装旧版 Blender**。release 资产与源码存档保留于 `target/tmp/gacha-setup/`（gitignored）。

剩余 PENDING（见 reference-toolchain.toml）：Blender 基线精确版本（待 addon 实测后锁定）、render engine / color management / view transform、export contract、profile genshin_schema、fixture/golden 相关修订号。

对 It-1a 后续步骤的阻塞更新：交付物 2（Toolchain Lock 定稿）的 upstream 侧证据已齐，可按 ADR-0011 流程提请维护者确认；§6 中"安装被锁旧版 Blender"预案**不再需要**。遗留事项：docs/reference/02-reference-toolchain-lock.md 中候选上游的仓库归属表述（"GitHub 仓库 michael-gh1/… 的 release 包"）与实际不符，建议走文档治理流程补记正确仓库 `PaoloESAN/gacha-setup`。
