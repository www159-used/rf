## Why

把已有 rf/rff shell 行为做成本地 macOS 可执行命令。用户最终明确只需要简单 CLI，本变更不包含 GUI 或 TUI。

## What Changes

- `rf [path]` 复制绝对路径文本，默认当前目录。
- `rf -f [path]` / `rff [path]` 复制文件引用；目录先打包为唯一 tar。
- 明确错误状态、特殊路径、归档取消和显式清理。
- macOS 原生剪贴板适配，验证写入进程退出后内容仍可读取。

## Capabilities

### New Capabilities

- `path-copy`: 命令行入口、路径解析、复制模式和输出。
- `archive-lifecycle`: 目录归档、取消、保留和清理。
- `platform-clipboard`: macOS 文件引用与退出后生命周期。

### Modified Capabilities

无。

## Impact

Rust 工程提供 rf、rff 两个命令。使用用户数据目录保存归档，不修改 shell 配置。Windows/Linux 留待后续变更。
