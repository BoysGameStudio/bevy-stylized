# License / Provenance Boundary

> **Status:** Normative architecture specification


当前 Gacha Setup 仓库声明 GPL-3.0，并明确其仓库不分发游戏模型、纹理或专有游戏资产。

因此公开 `bevy-stylized` 仓库必须区分：

```text
Behavioral / Visual Reference
vs
Copied Source Code
vs
Copied Assets
```

要求：

1. 不把未授权 HoYoverse/其它游戏资产提交到公开仓库；
2. 公共 Fixture 使用自制、CC0 或明确允许再分发的资产；
3. 复制/修改任何 GPL 上游源码前必须单独进行 License Review；
4. 不因为“只是 Shader”就默认上游代码可以复制进 permissive core；
5. `THIRD_PARTY_NOTICES.md` 记录实际使用的第三方代码、工具与许可；
6. Reference 文档记录来源与行为映射，但 Runtime Core 尽量保持独立实现；
7. 私有 Upstream Reference-Parity 资产如无法合法公开，可进入 optional local/private reference suite，不得成为公开复现的本地检查的必要依赖。

这不是为了削弱 Gacha Setup 的参考价值，而是保证：

```text
Reference Provenance 清晰
+
公开仓库可合法复用
+
Runtime License 边界明确
```

---
