## Context

项目来源是用户的 rf/rff shell 函数。用户最终将范围明确为简单本地 macOS CLI；先前扩展的 GUI、TUI 已移除。

## Goals / Non-Goals

**Goals:** 保留路径文本、文件引用、目录打包和默认当前目录语义，提供可直接执行的 macOS 命令。

**Non-Goals:** GUI、TUI、其他平台实现、多选、拖放和自动编辑 shell 配置。

## Decisions

- 使用 Rust 1.95.0，作为单一 rf-copy crate 发布，内部 core、clipboard、cli 模块分别负责路径/归档、平台接口、参数/退出状态。Cargo.lock 固定依赖；cargo install rf-copy --locked 安装 rf/rff 两个命令。
- CLI 使用 clap；文件模式由 `-f` 或 rff 入口指定。stdout 仅输出成功路径及换行，stderr 用于诊断和归档进度。退出码为 0/1/2/130。
- 路径解析使用系统 canonicalize，拒绝不存在、特殊文件或无法无损表示的路径。顶层链接解析到目标；内部归档链接保留。
- tar 在用户数据目录的独占 `rf-job-*` 目录生成，先写临时文件，完整结束后重命名为 `archive.tar`，随后提交剪贴板。内部保留源目录名作为顶层，根目录用 root。随机任务目录避免并发覆盖，存储目录排除避免归档自身。
- 支持分块取消；准备阶段失败或取消不碰剪贴板。提交期间不可取消。未提交的归档用 RAII 回收，成功后保留直到显式清理。
- `--clean-archives` 预览，增加 `--yes` 才删除；进程间归档锁禁止清理活动任务，清理不跟随链接且只处理本工具任务目录。
- macOS 通过 objc2 使用 NSPasteboard，文本写字符串，文件写 NSURL；文件写入后同步读回核对，防止短命 CLI 退出后数据不可读。剪贴板提交锁将 rf 进程的短暂写入/核对串行化。
- 默认数据目录为 `~/Library/Application Support/rf`；允许绝对路径 RF_DATA_DIR 隔离测试。

## Risks / Trade-offs

- 系统调用写入中失败 → 如实报错，不保证恢复旧剪贴板，不降级为文本。
- 源文件读取期间变化 → 可检测的变化报错，不声称提供快照或完整备份语义。
- 成功归档占磁盘 → 提供显式预览和清理，不自动失效。
- Finder 自动化权限不可用 → 格式读回与实际粘贴分开记录，保留人工验收。

## Validation

核心测试不写系统剪贴板；独立 macOS 冒烟对真实剪贴板进行退出后读取验证，并恢复运行前可读取的数据。具体结果与手动步骤见 `docs/macos-validation.md`。

## Cargo 分发

用户选择 crates.io 为分发渠道，包名确定为 rf-copy（rf 和 rf-cli 已被占用）。移除自定义 tar.gz 打包和 CI artifact 上传；补充 MIT LICENSE、发布元数据与文件白名单。发布前验证 cargo publish --dry-run、解包源码安装和真实剪贴板冒烟，上传后验证从 crates.io 安装。

## 英文默认界面

0.1.1 将 CLI 自身的帮助、提示、错误上下文及所有随包发布文档统一为英文；不引入语言选择机制。保留中文等 Unicode 路径测试，系统错误详情沿用操作系统提供的文本。按已确定的 crates.io 发布流程发布补丁版本。

## 许可证与源码仓库

0.1.2 按用户要求改用 GPL-3.0-only，LICENSE 保存完整 GPLv3 英文原文；已发布的旧版本保持原许可证。源码仓库为 https://github.com/www159-used/rf，使用 main 分支；GitHub 承载源码与 CI，安装分发继续使用 crates.io。本地 Codex 配置与构建产物不纳入 Git。
