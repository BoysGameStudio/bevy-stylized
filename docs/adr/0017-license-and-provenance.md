# ADR-0017: License and Provenance

- **Status:** Accepted
- **Decision owner:** Project maintainer
- **Date:** 2026-09-22

## Context

Do not preselect a permissive license before GPL/reference provenance review.

## Options

1. 开源宽松许可证（MIT OR Apache-2.0）。
2. Copyleft 许可证（GPL 系）。
3. 专有私有许可（保留所有权利，不分发）。

## Decision

选项 3。项目是**私有仓库、不分发**：不选择开源许可证，全部权利保留。
上游 GPL-3.0 工具（Gacha Setup、Festivity/Blender-miHoYo-Shaders）仅作为
**行为参考与制作工具链**使用：其输出物不被 GPL 覆盖（GPL 不传染程序输出；
Blender 官方同样声明 .blend/渲染产物归作者），且任何 GPL 源代码不得复制进
本仓库。游戏资产保留在 git-ignored `assets/local/`，绝不进入公开 fixture 或
提交历史。许可证选择的完整边界合同见
[license/provenance boundary](../reference/05-license-and-provenance-boundary.md)。

## Evidence

- 依赖许可证扫描（2026-09-22，Cargo.lock 全部 423 个 registry 包逐一读取
  缓存清单）：全部为宽松许可证（MIT/Apache-2.0/BSD/Zlib/CC0/Unlicense/
  BSL-1.0/ISC/OFL 及组合）；无 copyleft-only 依赖。唯二含 GPL/LGPL 选项的包
  均为多选一且存在宽松选项：`r-efi`（MIT OR Apache-2.0 OR LGPL-2.1-or-later，
  选 MIT/Apache）、`self_cell`（Apache-2.0 OR GPL-2.0-only，选 Apache-2.0）。
  两个平台门控包（pkg-config、wayland-backend）清单不在本地缓存，其上游为
  MIT OR Apache-2.0（Alex Crichton 系）——若未来分发需重新核对。
- 来源记录：genshin-odette 资产包 44 个文件带 SHA-256 与来源 URL
  （`assets/sources/genshin-odette.json`）；THIRD_PARTY_NOTICES.md 记录 GPL
  工具链与 Tenacity 复用清单（MIT）。
- 清洁室声明：WGSL shader 为独立实现；GPL shader 仅作行为对照（参考渲染
  对比链路的判定输入），无逐行移植。

## Consequences

- 私有状态下无任何开源义务；`publish = false` 与未设置 `license` 保持不变，
  仓库根新增 LICENSE 文件标明专有属性。
- **一旦计划任何形式的分发**（公开仓库、交付构建、捆绑资产的应用、商业授权），
  必须重新打开本 ADR 并完整审查：GPL 边界、依赖版权声明义务、参考产物
  （Blender 渲染图）的再分发权、以及目标的实际许可证选择。
- 本决定不构成对任何第三方资产权利的声明；游戏资产来源方的使用条款另行约束。

## Revisit trigger

计划任何形式的分发或开源；上游工具链许可变更；依赖树引入 copyleft-only 包。
