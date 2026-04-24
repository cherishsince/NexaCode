# NexaCode 功能清单与实现路线

## Phase 1: 项目脚手架与基础 TUI

### 1.1 项目初始化 ✅
- [x] 创建 Rust 项目结构
- [x] 配置 Cargo.toml 依赖
  - [x] ratatui
  - [x] crossterm
  - [x] tokio (full)
  - [x] serde + serde_json
  - [x] anyhow, thiserror
  - [x] tracing, tracing-subscriber
  - [x] figment (配置)
- [x] 创建目录结构（见 ARCHITECTURE.md）
- [x] 配置 logging 系统
- [x] 基础错误处理链

### 1.2 基础 TUI 框架 ✅
- [x] 实现 App 主状态结构体
- [x] 实现事件循环（Event Loop）
- [x] 实现渲染循环（Render Loop）
- [x] 基础布局管理
  - [x] 顶部状态栏
  - [x] 主内容区（对话区）
  - [x] 底部输入栏
- [x] 基础键盘事件处理
  - [x] Ctrl+C 退出
  - [x] q 退出（Normal 模式）
  - [x] 方向键导航

### 1.3 基础视图组件 ✅
- [x] ChatView（对话视图）
  - [x] 消息列表渲染
  - [x] 用户输入框
  - [x] 支持输入编辑
- [x] 主题切换（Dark/Light）

### 1.4 类 OpenCode 的 TUI 界面 ✅
- [x] Welcome 欢迎页
  - [x] ASCII Logo 展示
  - [x] 快捷操作提示
  - [x] Enter 进入聊天模式
- [x] Chat 聊天界面
  - [x] 左侧边栏（快捷命令、最近会话）
  - [x] 右侧聊天主区域
  - [x] 底部输入框
  - [x] Esc 返回欢迎页
- [x] 主题切换（Dark/Light）

---

## Phase 2: 状态管理与核心模块

### 2.1 状态管理系统 ✅
- [x] 定义 Action 枚举
- [x] 实现 Reducer 系统
- [x] 实现 Undo/Redo 栈
- [x] 状态变更通知机制

### 2.2 模式系统 ✅
- [x] Normal 模式
  - [x] 浏览消息
  - [x] 导航快捷键
- [x] Input 模式
  - [x] 文本输入
  - [x] 光标移动
  - [x] 历史输入记录（上下箭头）
- [x] Command 模式
  - [x] :quit / :q
  - [x] :help / :h
  - [x] :clear
  - [x] 命令历史
- [x] Search 模式
  - [x] 搜索消息
  - [x] n/N 跳转匹配

### 2.3 对话系统 ✅
- [x] Message 数据结构
  - [x] Role (User/Assistant/System/Tool)
  - [x] 内容
  - [x] 时间戳
  - [x] 元数据
- [x] 对话历史管理
  - [x] 内存存储
  - [ ] 持久化（可选）
- [x] 会话管理
  - [x] 当前会话
  - [x] 会话切换（可选）

---

## Phase 3: LLM 集成与 Agent 核心

### 3.1 LLM 客户端
- [ ] 定义统一 LLM trait
- [ ] 实现 Anthropic Claude 客户端
  - [ ] Messages API
  - [ ] 流式响应
  - [ ] Tool use 支持
- [ ] 配置管理
  - [ ] API Key 管理
  - [ ] 模型选择
  - [ ] 参数配置（temperature, max_tokens 等）

### 3.2 Agent Controller
- [ ] Agent State 状态机
  - [ ] Idle
  - [ ] Thinking
  - [ ] ExecutingTool
  - [ ] StreamingResponse
  - [ ] Error
- [ ] 核心推理循环
  - [ ] 接收用户输入
  - [ ] 构建上下文
  - [ ] 调用 LLM
  - [ ] 处理响应
  - [ ] 流式渲染

### 3.3 上下文管理
- [ ] Context Budget 管理
  - [ ] Token 计数
  - [ ] 上下文裁剪策略
- [ ] 消息历史管理
  - [ ] 滑动窗口
  - [ ] 重要消息保留

---

## Phase 4: MCP 协议与工具系统

### 4.1 MCP 协议基础
- [ ] MCP 消息类型定义
  - [ ] JSON-RPC 2.0 基础
  - [ ] Requests/Notifications/Responses
- [ ] Transport 层
  - [ ] Stdio transport
  - [ ] (可选) WebSocket transport

### 4.2 MCP 客户端
- [ ] MCP Server 连接管理
  - [ ] 启动子进程
  - [ ] 进程生命周期管理
- [ ] MCP 协议实现
  - [ ] Initialize 握手
  - [ ] Ping/Pong
  - [ ] 能力协商

### 4.3 工具系统
- [ ] Tool 数据结构
  - [ ] Name
  - [ ] Description
  - [ ] InputSchema (JSON Schema)
- [ ] 内置工具实现
  - [ ] `read_file` - 读取文件
  - [ ] `write_file` - 写入文件
  - [ ] `edit_file` - 编辑文件（搜索替换）
  - [ ] `list_dir` - 列出目录
  - [ ] `run_command` - 运行命令
  - [ ] `git_status` - Git 状态
  - [ ] `git_diff` - Git diff
- [ ] 工具执行沙箱
  - [ ] 安全检查
  - [ ] 超时控制
  - [ ] 输出捕获
  - [ ] 变更追踪

### 4.4 资源与提示词（可选，后续扩展）
- [ ] Resources 读取
- [ ] Prompts 模板

---

## Phase 5: Skills 系统

### 5.1 Skill 核心
- [ ] Skill 数据结构
  - [ ] ID/Name/Description
  - [ ] Version
  - [ ] Tags
- [ ] Trigger 系统
  - [ ] CommandTrigger ("/skill-name")
  - [ ] SemanticTrigger (语义匹配)
  - [ ] FilePatternTrigger (文件模式)
  - [ ] EventTrigger (事件触发)

### 5.2 Skill 定义类型
- [ ] PromptSkill - 提示词模板
- [ ] CompositeSkill - 工具组合
- [ ] PipelineSkill - 流水线
- [ ] CustomSkill - 自定义逻辑（预留）

### 5.3 Skill Manager
- [ ] Skill Registry
  - [ ] 注册/查询
  - [ ] 加载/卸载
- [ ] Skill 发现
  - [ ] 内置 Skills
  - [ ] 用户 Skills (~/.claude/skills)
  - [ ] 项目 Skills (.claude/skills)
- [ ] Skill 执行引擎
  - [ ] 触发匹配
  - [ ] 参数解析
  - [ ] 执行调度

### 5.4 内置 Skills
- [ ] `/commit` - 智能提交信息生成
  - [ ] 读取 git diff
  - [ ] 生成提交信息
  - [ ] 确认后执行 commit
- [ ] `/review` - 代码审查
  - [ ] 读取变更文件
  - [ ] 生成审查意见
- [ ] `/explain <file>` - 代码解释
  - [ ] 读取文件
  - [ ] 生成解释
- [ ] `/refactor <pattern>` - 重构
  - [ ] 分析代码
  - [ ] 生成重构计划
  - [ ] 执行重构
- [ ] `/test` - 生成/运行测试
  - [ ] 查找相关测试
  - [ ] 生成测试代码
  - [ ] 运行测试
- [ ] `/docs` - 生成文档
  - [ ] 分析代码
  - [ ] 生成文档

---

## Phase 6: 项目上下文与文件系统

### 6.1 文件系统集成
- [ ] FilesView（文件树视图）
  - [ ] 目录树渲染
  - [ ] 展开/折叠
  - [ ] 文件图标
- [ ] 文件监听
  - [ ] 监听文件变更
  - [ ] 刷新通知
- [ ] 文件缓冲区管理
  - [ ] 打开文件列表
  - [ ] 未保存变更标记

### 6.2 项目索引
- [ ] 项目扫描
  - [ ] 识别项目类型
  - [ ] 读取配置文件
- [ ] 符号索引（可选）
  - [ ] LSP 集成
  - [ ] 符号搜索

### 6.3 Git 集成
- [ ] Git 状态显示
- [ ] Git diff 查看
- [ ] Git 操作封装
  - [ ] commit
  - [ ] checkout
  - [ ] branch
  - [ ] ...

---

## Phase 7: 终端集成与高级视图

### 7.1 嵌入式终端
- [ ] TerminalView
  - [ ] PTY 集成（portable-pty）
  - [ ] 终端渲染
  - [ ] 输入转发
- [ ] Shell 集成
  - [ ] 命令执行
  - [ ] 输出捕获
  - [ ] 历史记录

### 7.2 任务视图
- [ ] TaskView
  - [ ] 任务列表
  - [ ] 任务状态（Pending/InProgress/Completed/Error）
  - [ ] 任务日志
- [ ] Task Queue 管理
  - [ ] 任务创建
  - [ ] 任务取消
  - [ ] 任务依赖

### 7.3 多标签页/面板
- [ ] 标签页管理
  - [ ] 创建/关闭标签
  - [ ] 标签切换
- [ ] 面板分割（可选）
  - [ ] 水平分割
  - [ ] 垂直分割

---

## Phase 8: 规划引擎

### 8.1 规划核心
- [ ] Plan 数据结构
  - [ ] Steps（步骤）
  - [ ] Dependencies（依赖）
  - [ ] Status（状态）
- [ ] 规划生成
  - [ ] 需求分析
  - [ ] 任务分解
  - [ ] 依赖排序

### 8.2 计划执行
- [ ] 分步执行
- [ ] 执行监控
- [ ] 错误处理与重试
- [ ] 计划调整

### 8.3 计划可视化
- [ ] 计划树展示
- [ ] 执行进度
- [ ] 变更预览

---

## Phase 9: 配置与持久化

### 9.1 配置系统
- [ ] 配置文件结构
  - [ ] 全局配置 (~/.claude/config.json)
  - [ ] 项目配置 (.claude/config.json)
- [ ] 配置项
  - [ ] LLM 配置
  - [ ] MCP Servers 列表
  - [ ] UI 配置
  - [ ] Skill 配置

### 9.2 历史持久化
- [ ] 对话历史保存
- [ ] 会话管理
- [ ] 历史搜索

### 9.3 数据目录
- [ ] 建立 ~/.claude 目录结构
  - [ ] ~/.claude/config.json
  - [ ] ~/.claude/skills/
  - [ ] ~/.claude/sessions/
  - [ ] ~/.claude/logs/

---

## Phase 10: 完善与优化

### 10.1 用户体验
- [ ] 主题支持（暗色/亮色）
- [ ] 快捷键自定义
- [ ] 帮助界面
- [ ] 教程/引导

### 10.2 性能优化
- [ ] 大文件处理优化
- [ ] 虚拟滚动
- [ ] 延迟加载

### 10.3 可观测性
- [ ] 详细日志
- [ ] 性能追踪
- [ ] 错误报告

### 10.4 测试
- [ ] 单元测试
- [ ] 集成测试
- [ ] E2E 测试

---

## 依赖项参考

### 核心 Crate
```toml
# TUI
ratatui = "0.26"
crossterm = "0.28"

# Async
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Config
figment = "0.10"

# Filesystem
walkdir = "2.0"
notify = "6.0"

# Git
git2 = "0.19"

# Terminal
portable-pty = "0.8"

# Misc
tui-textarea = "0.4"  # 文本编辑
unicode-width = "0.1" # 字符宽度计算
```

---

## 实现顺序建议

### MVP (最小可用产品) - Phase 1 + 2 + 3
能进行基础对话，展示回复。

### 核心功能 - Phase 4 + 5
具备工具调用和 Skills 能力。

### 完整产品 - Phase 6 + 7 + 8 + 9 + 10
完整的 IDE 体验。
