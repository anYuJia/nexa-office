# Nexa Office

[English](README.md) · 简体中文

> 使用 Rust 构建的原生、轻量办公套件。

Nexa Office 的目标不是把网页编辑器套进桌面壳，而是从底层做一个真正原生的 Office 替代方案：

- **原生轻量**：Rust + Slint，不使用 Chromium、Electron、Tauri 或 WebView 编辑器运行时。
- **兼容优先**：DOCX / XLSX / PPTX 作为一等格式，未知 OOXML 内容尽量保留；无法安全改写时宁可阻止保存，也不静默破坏文件。
- **性能门禁**：启动、空闲 CPU、内存、文档操作延迟和安装体积都进入 CI，而不是最后再优化。
- **中文友好**：支持跟随系统、简体中文、English，设置会保存在本机。

## 当前进度

已完成：

- Phase 0：工程规范、AI 约束、代码/测试/性能/安全门禁
- Phase 1：原生 Rust/Slint 应用壳
- Phase 2：OOXML / OPC 基础层
- Phase 3：Nexa Docs MVP
- Phase 4：Nexa Sheets MVP
- Phase 5：Nexa Slides MVP
- Phase 6：兼容性与互操作强化
- Phase 7：原生产品集成
- 原生 UI/UX 现代化与中英文适配

Nexa Docs 当前已经具备真实 DOCX 工作流：

- 打开、保存、原子化另存为
- 段落编辑与插入
- 粗体 / 斜体 / 下划线
- 查找与全部替换
- 撤销 / 重做
- 分页与文档状态
- 兼容性检测与危险写入阻止
- 未修改 OPC Part 的原始压缩数据保留

> Docs、Sheets 与 Slides 都采用安全 MVP 边界，并不宣称完整复刻 Microsoft Office。超出当前安全写入范围的复杂 OOXML 会被识别并阻止破坏性保存。

Nexa Sheets 当前已经具备真实 XLSX 工作流：

- 1,048,576 × 16,384 理论边界下的稀疏单元格模型
- 固定大小虚拟化网格，不为百万行创建百万个 UI 对象
- 数值、文本、布尔、错误值与公式
- SUM / AVERAGE / MIN / MAX、算术、单元格/区域引用与循环引用检测
- 确定性重算
- 粗体、斜体、填充、对齐等基础格式
- 合并单元格、行高、列宽、冻结窗格
- 稀疏排序与筛选
- 多工作表
- XLSX 打开、原子保存、共享字符串导入与兼容性阻止

Nexa Slides 当前已经具备原生 PPTX 工作流：

- 多幻灯片语义模型与 16:9 原生工作区
- 文本框、基础形状、表格与图片关系保留
- 页面新增、复制、删除、前后切换
- 元素选择、文字编辑、删除与层级调整
- PPTX 打开、原子保存与重新打开
- 对动画、图表、OLE、音视频等暂不支持结构执行兼容性阻止，避免破坏性保存

## UI / UX

界面采用轻量、低卡片化的信息层级：

- 首页把“打开 Office 文件”作为一级入口，并显示带 DOCX / XLSX / PPTX 类型标识的最近文件
- 统一侧栏与显式紧凑导航模式；启用紧凑导航时同步压缩编辑器工具栏，并保留完整辅助功能标签
- Docs / Sheets / Slides 共享稳定的原生桌面导航、文件状态和兼容性语义
- 中英文完整切换，系统对话框、剪贴板、打印、恢复与拖入文件的失败反馈也提供中文前缀
- 自定义按钮、开关、导航、最近文件与紧凑符号按钮提供辅助功能角色与完整标签
- CJK 使用平台字体回退，文本编辑继续使用 Slint 原生输入控件以保持平台 IME 链路
- 设置与最近文件仅保存在本机，不依赖账号和云端

## 性能

Phase 3 在 GitHub 托管 Ubuntu Runner 的 Release 趋势样本中：

- 20 页 DOCX 全应用 PSS：约 **15.5 MiB**
- Private memory：约 **13.5 MiB**
- 空闲 CPU：采样 **0.000%**
- Release 可执行文件：约 **14.9 MiB**

Phase 4 Sheets 的 8,000 行 / 32,005 个已填充单元格稀疏样本中，模型估算约 **3.57 MiB**，240 单元格虚拟视口提取约 **266 µs**，重算约 **8.6 ms**，XLSX 保存约 **34 ms**，重新打开约 **53 ms**，benchmark 峰值 RSS 约 **28 MiB**。

这些数据用于 CI 趋势和回归门禁，不代表所有终端设备上的固定数值。

## 开发

要求 Rust 1.98.1：

```bash
cargo run --locked -p nexa-app
cargo test --workspace --all-targets --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

## 目录

- `crates/nexa-app`：原生应用与 Slint UI
- `crates/nexa-core`：UI 无关的应用状态
- `crates/nexa-docs`：DOCX 语义、编辑与布局
- `crates/nexa-sheets`：XLSX 工作簿、公式、虚拟化与 SpreadsheetML
- `crates/nexa-slides`：PPTX 演示文稿模型、PresentationML 与 Slides 编辑命令
- `crates/nexa-ooxml`：OOXML / OPC / ZIP / XML 基础层
- `docs/`：架构、门禁、路线图、性能与阶段验收记录

## 后续路线

Phase 8 发布硬化工程基线已完成：统一重写风险审计、Unicode/CJK/RTL 保真、原生文件对话框、最近文件、崩溃恢复、打印/剪贴板交接、三平台打包、构建元数据、校验和与发布门禁均已落地。Alpha / Beta / 1.0 的正式发布仍由发布清单、签名/公证、最终版本号和参考硬件验证决定。

详细计划见 [Roadmap](docs/ROADMAP.md)。

## 核心原则

> 正确性第一，经过测量的性能第二，功能数量第三。

任何会破坏文件、静默丢失 OOXML、造成无边界内存增长或绕过质量门禁的功能，都不算“完成”。
