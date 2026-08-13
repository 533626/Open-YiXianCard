export const WINDOWS_REPLAY_DATA_PATH =
  String.raw`%USERPROFILE%\AppData\LocalLow\DarkSunStudio\YiXianPai`;

export const LINUX_REPLAY_DATA_PATH =
  String.raw`$HOME/.config/unity3d/DarkSunStudio/YiXianPai`;

export const USER_AGENT_REPLAY_IMPORT_PROMPT = String.raw`# 帮我找到弈仙牌本机对局

目标
- 只定位我这台 Windows 或 Linux 电脑上的《弈仙牌》对局缓存。
- 不上传、修改、移动或删除任何原版文件。
- 找到后告诉我应该在 Open-YiXianCard 中选择哪个文件夹或 .bin 文件。

允许检查的范围
Windows:
%USERPROFILE%\AppData\LocalLow\DarkSunStudio\YiXianPai

Linux:
$HOME/.config/unity3d/DarkSunStudio/YiXianPai

缓存结构
userLocalDatas/<用户目录>/recentBattleDatas/*.bin
userLocalDatas/<用户目录>/downloadBattleDatas/*.bin
userLocalDatas/<用户目录>/starBattleDatas/*.bin

处理步骤
1. 先按当前操作系统确认对应 YiXianPai 目录是否存在；不要扫描整块磁盘。
2. 按修改时间列出最新的 .bin，对路径、大小和修改时间保留原格式。
3. 如果我提供了战绩码，优先检查 downloadBattleDatas；若本机没有对应记录，提醒我先在原版客户端打开该战绩，使其写入下载缓存。
4. 不要猜测或改写二进制内容，也不要把工程 fixture 当成玩家对局。
5. 最终只给我两种安全选择：
   - 在网页点击“选择弈仙牌文件夹”，选择 YiXianPai 目录；
   - 或把确认过的单个 .bin 拖入网页。

PowerShell 只读检查
$root = Join-Path $env:USERPROFILE 'AppData\LocalLow\DarkSunStudio\YiXianPai'
Get-ChildItem -LiteralPath $root -Recurse -File -Filter *.bin |
  Where-Object { $_.DirectoryName -match '\\(recentBattleDatas|downloadBattleDatas|starBattleDatas)$' } |
  Sort-Object LastWriteTime -Descending |
  Select-Object -First 30 FullName, Length, LastWriteTime

Bash 只读检查
root="$HOME/.config/unity3d/DarkSunStudio/YiXianPai"
find "$root/userLocalDatas" -type f -name '*.bin' -printf '%T@ %p\n' 2>/dev/null |
  sort -nr |
  head -n 30

隐私边界
- 不读取 YiXianPai 目录之外的文件。
- 不输出玩家昵称、账号标识或二进制内容。
- 所有操作保持只读；需要扩大范围时先问我。`;
