## ADDED Requirements

### Requirement: macOS 提供真实文件引用
系统 SHALL 通过 NSPasteboard 写入文件 URL 对象，表达复制而非剪切，不以路径文本替代文件引用并报告成功。

#### Scenario: 文件引用格式
- **WHEN** 复制包含中文、空格、引号或 URI 特殊字符的普通文件
- **THEN** 系统剪贴板提供对应的原生文件 URL，源文件保持不变

### Requirement: CLI 退出后仍能读取剪贴板
系统 SHALL 在提交成功后返回，CLI 退出后文本和文件引用仍可重复读取，直到被其他复制替换、会话结束或引用文件被删除。

#### Scenario: 退出后读取
- **WHEN** rf 文件模式成功退出
- **THEN** 独立进程能够重复读回与本次目标路径相同的文件 URL

### Requirement: 文件提交必须验证
系统 SHALL 在文件引用写入后核对原生格式，不仅根据写入 API 返回值报告成功；rf 进程之间 SHALL 串行化提交和核对，避免并发干扰。失败 SHALL 返回非零状态且不输出成功路径。

#### Scenario: 系统拒绝写入
- **WHEN** 平台 API 写入或读回核对失败
- **THEN** 命令返回 1 并报告错误，不退化成路径文本复制

### Requirement: 平台验收区分读回与实际粘贴
项目 SHALL 单独记录构建、格式读回和 Finder 实际粘贴结果；未完成的实际粘贴 SHALL 保持未验证。

#### Scenario: 缺少自动化权限
- **WHEN** 系统拒绝 Finder 自动化所需权限
- **THEN** 记录格式测试结果并提供人工粘贴步骤，不声称 Finder 验收通过
