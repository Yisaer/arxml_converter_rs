# 仓库规范

## 项目简介
本项目是 [arxml-converter](https://github.com/yisaer/arxml-converter)（Go 版本）的 Rust 重写实现，用于解析 AUTOSAR CP（Classic Platform）ARXML 文件并将其中定义的数据类型映射为可执行解码逻辑。

### 整体定位
本 crate 最终服务于 [veloFlux](https://github.com/yisaer/veloFlux) 流处理 pipeline 的 stream format 字段：当 pipeline 消费二进制线缆数据（如 SOME/IP 报文）时，根据 ARXML 中定义的数据类型 schema 对 `&[u8]` 进行零拷贝解码，产出 pipeline 可消费的结构化数据。

```
ARXML 文件 → arxml_converter_rs → 类型 schema + decoder → veloFlux stream format → pipeline 解析二进制数据
```

关键结论：
- **这不是一个纯 parser/转换工具**——它需要产出可执行的解码器，不仅仅是中间表示。
- **第一阶段只做 CP**（Classic Platform），因为 CP 包含完整的 PDU → Signal → DataType 映射链路，是线缆格式的权威来源。AP（Adaptive Platform）作为 deferred/future work。
- **产出类型需与 veloFlux 原生类型对齐**——优先兼容 veloFlux 的 `Collection`/`Tuple`/`Value` 数据体系，避免 pipeline 中二次类型转换。

## 开发策略
以端到端可验证的 decode 流程为主线推进，而非按模块线性开发：
1. 先定义解码器产出的目标数据结构（与 veloFlux 对齐）
2. 手工构造一个已知类型的 ARXML 片段 + 一段二进制数据，写死解码逻辑先跑通端到端流程
3. 回头将 ARXML 解析接进来，替代手工数据
4. 补充完整的 CP 解析链路（DataTypes、Topology、Communication、System、SoftwareTypes）
5. 性能优化（零拷贝解码、inline 等）

AP（Adaptive Platform）支持作为 deferred/future work。

## 文档与设计背景
- 本项目的功能设计背景与方案沉淀在 `docs/` 目录的 Markdown 文档中；在了解某个功能的背景/约束/方案时，优先查阅对应文档。
- Go 版本的实现细节可作为设计参考，但 Rust 版本无需逐行对译——应利用 Rust 的类型系统与所有权模型进行更安全、更清晰的实现。
- 在 `docs/` 中新增文档时，语法（syntax）/类型映射相关的内容需要单独分类/区分，便于后续查找与维护。

## 构建、测试与开发命令
- `make build` — 以 debug 模式编译项目。
- `make release` — 构建优化后的二进制产物到 `target/release/`。
- `make test` — 运行项目所有测试。
- `make fmt`, `make clippy` — 执行格式化与 lint（`-D warnings`）。

## 代码风格与命名约定
- 运行 `make fmt`，遵循 Rustfmt 默认风格（4 空格缩进、尾随逗号）。
- 代码、注释与标识符统一使用英文；使用 `snake_case` 命名条目、`CamelCase` 命名类型、`SCREAMING_SNAKE_CASE` 命名常量。
- 遵循模块边界：parser 负责 XML 解析与数据提取，ast 定义中间表示类型，converter 负责将 AST 转换为 IDL，util 提供 XML/类型辅助工具。

## 项目架构

第一阶段（CP only）：

```
arxml_converter_rs/
├── src/
│   ├── lib.rs              # 库入口，暴露公开 API（如 load_arxml、resolve_type、decode 等）
│   ├── ast/                # 核心类型定义（DataType, Structure, Array, Vector, TypReference 等）
│   │   ├── mod.rs
│   │   ├── types.rs        # DataType, TypReference, Array, Vector, Structure, StructureTypRef
│   │   └── resolver.rs     # 类型引用解析与类型名称映射（uint8→OctetType 等基本类型映射）
│   ├── parser/             # ARXML 解析（CP only，AP 为 deferred）
│   │   ├── mod.rs
│   │   ├── datatypes.rs    # DataTypes 包解析
│   │   ├── topology.rs     # Topology 包解析（PDU Triggering, Header ID, Service ID 映射）
│   │   ├── communication.rs# Communication 包解析（PDU Ref, Signal Ref）
│   │   ├── system.rs       # System 包解析（Operation Ref）
│   │   └── software_types.rs# SoftwareTypes 包解析（Interface Ref）
│   ├── decoder/            # 二进制解码器（核心产出）
│   │   ├── mod.rs
│   │   ├── codec.rs        # Decode trait 定义与各基础类型的解码实现
│   │   └── structured.rs   # STRUCTURE/ARRAY/VECTOR 复合类型的解码实现
│   └── util/               # 辅助工具
│       ├── mod.rs
│       └── xml.rs          # XML 元素查询（get_shortname, get_category, get_elements 等）
├── tests/                  # 集成测试
│   └── test_data/          # 测试用 ARXML 文件 + 对应二进制数据
├── docs/                   # 设计文档
├── Makefile
├── Cargo.toml
└── AGENTS.md
```

注：AP（Adaptive Platform）支持为 deferred/future work，届时在 `parser/` 下新增 `ap/` 子模块。

### 核心类型关系
- `DataType` — ARXML 中数据类型的统一表示，根据 `Category` 区分：`TYPE_REFERENCE`（基本类型引用）、`ARRAY`、`VECTOR`、`STRUCTURE`
- 解析器（CP）负责从 XML 中提取 `DataType` 并存入 map
- `decoder` 模块根据 `DataType` 生成对应类型的二进制解码逻辑
- 类型名称映射规则（如 `uint8` → `OctetType`、`float` → `FloatType` 等）维护在 `ast/resolver.rs`

### 模块职责边界
- `parser` 模块：只负责 XML 解析，产出 `DataType` map 和 CP 查找链路所需的各种中间映射（serviceID→PDU、headerID→信号等），不依赖 `decoder`。
- `ast` 模块：定义核心数据类型（`DataType`、`Structure` 等）和类型解析逻辑。不与 veloFlux 耦合。
- `decoder` 模块：根据 `DataType` 生成/执行二进制解码，产出解码后的数据结构。**解码产出的值类型需与 veloFlux 的 `Collection`/`Tuple`/`Value` 体系兼容**，做到零开销或接近零开销转换。
- `util` 模块：纯函数工具（XML 元素查询、字符串→数值转换），不依赖项目内其他模块。

### 外部依赖
- XML 解析：优先使用 `roxmltree`（轻量、只读、零拷贝），符合本项目只读解析的使用场景
- 二进制编解码：优先使用标准库或轻量 crate（如 `byteorder`、`bytes`）实现，避免引入重型序列化框架
- veloFlux 类型对齐：解码产出类型应设计为可被 veloFlux pipeline 直接消费，或在 veloFlux 侧通过薄 adapter 层桥接

## 提交前自检守则

每次代码修改完成后、提交前，必须完成以下自检步骤。这些步骤来自历史 PR review 中反复出现的返工模式（遗漏调用点、文档漂移、修复不治本等），目的是在 reviewer 看到代码之前先由自己消灭低级问题。

### 变更影响面分析
- 当删除或重命名一个 public API（函数、类型、trait、enum variant、struct 字段）时，必须用 `rg`（ripgrep）全局搜索该名称，确保没有残留的调用点或文档引用。
- 对于「删除型」变更，必须额外检查：
  - 所有 `tests/` 目录下的集成测试
  - `docs/` 下所有 Markdown 文档
- 搜索时不要只搜精确匹配——考虑变体：snake_case / CamelCase、路径分隔符差异。

### 编译与测试验证
- 修改 public API 后，必须运行受影响的测试目标的编译检查：`cargo check --test <target>`，确认没有编译错误后再提交。
- 如果使用了 sed 或脚本批量修改，必须在修改后运行编译检查——sed 不生效是真实发生过的教训。

### 文档同步
- 当修改了某个功能的运行时行为（API 返回值、错误处理策略、类型映射规则等），必须同步检查并更新 `docs/` 下相关文档中对该行为的描述。代码实现是最终的真相来源，文档必须跟随代码。
- 新增功能时，文档中描述的行为必须是当前 PR 实际实现的行为。不得在文档中描述尚未实现的设计规划，除非明确标注为 deferred/future work。
- 删除功能时，必须删除或重写文档中对该功能的所有描述，不得留下孤立的章节或残缺的句子。

### 修复方式
- 面对 reviewer 指出的问题，要从根因层面修复，而不是在具体指出的路径上打补丁。
- 如果正确的修复方案工作量较大，需要在 commit message 中说明当前方案的局限性，并在 `docs/` 中明确标注为 deferred/future work，不得假装问题已解决。

## 沟通原则
- 与维护者沟通时统一使用中文（需求澄清、方案确认、评审反馈、变更说明等）。

## 编码守则
- 每次写代码前，必须先将代码修改的设计方案与我确认；未经确认不得直接改代码。
- 每次修改完 Rust 代码后，必须立即运行 `make fmt` 与 `make clippy` 检查，确认通过后才能继续下一步。
- 修改代码时只能在当前分支上进行，严禁切换分支；所有改动都在用户当前所处的分支上完成。
- 不用关心 `git status`，交给用户自己操作。
- 开发完成的代码必须通过 `make fmt` 与 `make clippy`；提交/合并前优先用这两个命令自检（clippy 以 `-D warnings` 运行）。
- 任何代码注释以及 `docs/` 下的文档编写均应使用英文。
- 对同一 struct 的同一职责，尽量维护单一方法入口；允许但克制使用 `with_*` 变体，避免因少量参数差异扩散出多个近似方法。需要可选项/扩展点时，优先使用 Options/Config struct、枚举参数，或独立的 builder struct。
- 当需要我们生成 PR title 和 PR description 时，PR title 必须通过 CI 的 `PR Title Lint`（Conventional Commits 规范）。
  - 格式：`<type>(<scope>): <subject>`（`(<scope>)` 可选；breaking change 可用 `!`）。
  - `type`/`scope` 使用小写；常用 `type`：`feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert`。
  - PR title 和 PR description 必须使用英文撰写。

## 测试守则
- 不要求每次修改代码后都运行 `make test` 做全量验证；优先运行与改动相关的最小测试集合（例如仅运行新增/受影响的单元测试）。
- 如需新增单元测试：先向我确认该单元测试的验证用例（case）与预期结果，再进行实现与运行。
- 测试 ARXML 解析器时，优先使用小型、自包含的 ARXML 片段作为测试输入，避免依赖大型外部文件。
- CP 解析相关测试关注数据类型映射链路（serviceID → headerID → PDUTRIGGERINGREF → ISignalIPDU → ... → TypeRef）的完整性。
- `decoder` 模块测试为最高优先级：每条测试包含「一段已知 ARXML 类型定义 + 一段对应二进制数据 + 预期解码结果」，确保端到端可验证。
- 注：AP 相关测试规则保留，待 AP 实现时启用。

## 各模块开发守则
- `parser` 模块：负责将 ARXML 解析为 AST 数据结构；解析结果存放在 `DataType` map 和 CP 查找链路所需的各种中间映射中，作为后续模块交互的契约。
- `ast` 模块：定义 AUTOSAR 类型系统的中间表示，以及到基本类型和复合类型的解析逻辑。类型名称映射规则维护在此模块。不应依赖 veloFlux 的任何类型。
- `parser` 模块（CP 查找链路）：沿 serviceID → headerID → PDUTRIGGERINGREF → ISignalIPDU → ISIGNALREF → CommunicationPDURef → SystemSignalRef → OperationRef → InterfaceRef → TypeRef 链路查找目标数据类型。链路中的任何一环缺失都应返回明确错误，以便排查 ARXML 数据完整性问题。
- `decoder` 模块：核心性能敏感路径。对 `&[u8]` 的解析应尽可能零拷贝（借用原始数据），避免不必要的 `Vec<u8>` 分配。产出类型需设计为可被 veloFlux pipeline 消费。
- `decoder` 模块的「解码表」：解析阶段将 `(serviceID, headerID)` 到 `DataType` 的映射预编译为快速查找结构（如 `HashMap`），避免运行时每次都走完整查找链路。
- `util` 模块：提供 XML 元素查询辅助函数和类型转换工具。纯函数，无状态，无模块间依赖。
