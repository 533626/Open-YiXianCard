<!-- topic: webui -->

# 玩家本机对局导入

公开站点把原版对局视为用户显式选择的本地输入。网站不自动扫描磁盘、不上传对局，也不把
工程 fixture 暴露成玩家功能。

## Windows 默认位置

```text
%USERPROFILE%\AppData\LocalLow\DarkSunStudio\YiXianPai
```

## Linux 默认位置

```text
$HOME/.config/unity3d/DarkSunStudio/YiXianPai
```

该目录下的对局缓存位于：

```text
userLocalDatas/<用户目录>/recentBattleDatas/*.bin
userLocalDatas/<用户目录>/downloadBattleDatas/*.bin
userLocalDatas/<用户目录>/starBattleDatas/*.bin
```

生产 UI 统一使用“导入对局”入口：

1. “战绩码”在用户授权的缓存目录中匹配原版展示码或短码。
2. “本机记录”读取用户选择的 `YiXianPai` 文件夹或拖入的 `.bin`，在浏览器 Worker 中解码。
3. “对局包”兼容项目定义的版本化 JSON；这是高级交换格式，不是默认玩家流程。

原版一份 `RecentBattleInfo` 可以包含多轮斗法；产品必须展示轮次、双方角色、先手、胜者与终局
差值，让用户明确选择导入哪一轮。来自本机的原版结果仍标记“未认证”，不能冒充仓库准入证据。

## 让用户自己的 AI 助手协助

导入面板提供“复制给 AI 助手”按钮。说明必须保留标题、列表、Windows/Linux 路径及
PowerShell/Bash 代码格式，并
约束助手只检查上述目录、保持只读、不输出玩家标识。助手的职责是帮助用户定位目录或文件；
二进制解码仍由本站 Worker 完成，不能要求普通用户安装仓库的 Python、Bun 或 fixture 工具链。

若战绩码在缓存中不存在，应让用户先在原版客户端打开该战绩，使其落入
`downloadBattleDatas`，再回到网页重新选择目录。
