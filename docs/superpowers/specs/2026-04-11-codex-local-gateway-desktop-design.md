# Codex 本地网关桌面端设计

- 日期: 2026-04-11
- 状态: 已完成设计草案，待用户审阅书面 spec
- 项目: CodexLAG

## 1. 背景

目标是使用 Tauri 2 开发一个 Windows 优先的本地 Codex 网关桌面软件。该软件既是一个桌面控制台，也是一个仅监听本机回环地址的本地 API 网关。它需要管理 Codex 官方账号和第三方中转站，按照可配置的优先级和降级策略为本机上的 Codex 客户端选路，并提供额度、日志、token 和费用统计视图。

该产品的首版不是通用 OpenAI 代理，也不是远程多租户云平台。它的核心是本地控制面加本地数据面：

- 控制面负责账号、中转、平台 key、策略、健康状态、余额刷新和日志查询
- 数据面负责接收本机请求、校验平台 key、按策略选路、转发请求、记录请求链路和用量

参考项目 `CLIProxyAPI` 可借鉴路由策略、usage 统计和管理接口的思路，但本项目不直接复刻其服务端形态。首版的交付形态是单机桌面应用。

对于官方特性支持，本项目只跟随 `CLIProxyAPI` 中已经存在的实现，不为桌面版单独新增官方特性语义。

当前可明确借鉴的模式包括：

- 基于模型注册信息的能力补齐与约束，例如最大 completion token 或上下文相关能力
- 通过专用入口或专用参数对上游特性进行透传，例如 `compact` 类请求路径

## 2. 已确认范围

以下边界已经在设计阶段确认：

- 首版范围包含平台 key 体系，不仅仅是本地代理
- 本地网关对外协议优先服务 Codex 使用场景，而不是先做 OpenAI 兼容代理
- 官方账号同时支持导入现有登录态和应用内登录
- 降级策略为可配置策略，不限于简单硬失败切换
- 平台 key 支持每个 key 自定义规则，而不是固定模式
- 敏感凭据使用 Windows 系统级安全存储，不落地明文数据库
- 官方账号额度展示只接受真实可查值，查不到就明确标记不可查询
- 官方特性能力只跟随 `CLIProxyAPI` 已有实现提供，不额外为桌面版单独发明新特性
- 本地网关只监听 `127.0.0.1` 或 `localhost`
- 首版目标系统仅为 Windows
- 首次启动自动创建一个 `default key`
- 需要一个自定义托盘右键面板，用于 `default key` 模式切换和状态摘要

## 3. 产品目标

### 3.1 功能目标

V1 需要完成以下能力：

- 管理 Codex 官方账号
- 管理第三方中转站，首批支持 `newapi`
- 查看官方账号真实剩余额度
- 查看支持余额接口的第三方中转站额度
- 为每个账号和中转设置优先级、启用状态和标签
- 为本地客户端创建平台 key
- 为每个平台 key 绑定允许模式和独立路由策略
- 按策略在官方账号池和中转池之间选路与降级
- 展示并透传 `CLIProxyAPI` 已有的官方特性能力，不额外扩展桌面版专属官方特性
- 记录请求日志、每次尝试链路、token 统计和预估费用
- 提供 `default key` 的首启生成与托盘快捷切换

### 3.2 非目标

V1 明确不包含以下内容：

- 局域网或公网共享节点能力
- 多租户远程平台
- 复杂负载均衡和并行竞速请求
- 所有中转站的通用余额适配
- 不能提供真实额度时的官方额度估算
- 完整 UI 自动化测试体系

## 4. 推荐架构

推荐采用单进程桌面控制面加内嵌本地网关的结构。

### 4.1 方案选择

推荐方案：

- Tauri 2 应用作为唯一宿主进程
- Rust 后端同时承载本地 HTTP 网关、控制面服务、运行时状态和持久层
- Web 前端负责管理界面

选择理由：

- 部署简单，符合单机使用场景
- 系统级凭据存储更容易与 Windows 集成
- 托盘、窗口、命令调用、后台任务都可以由 Tauri 统一管理
- 虽然是单进程，但内部模块边界按可拆分守护进程的方式设计，便于未来演进

### 4.2 模块划分

系统拆分为以下模块：

1. `Desktop UI`
负责账号、中转、平台 key、策略、日志、统计和状态展示。

2. `Control Plane`
负责配置校验、配置变更、策略解析、余额刷新、连接测试、健康状态查询和对前端提供 Tauri commands。

3. `Loopback Gateway`
仅监听本机回环地址，供本机 Codex 客户端调用。负责平台 key 校验、请求归一化、选路、转发和写日志。

4. `Routing Engine`
根据平台 key 绑定的策略，在官方账号池和中转池中选出当前可用节点，支持优先级、熔断、恢复和跨池降级。

5. `Provider Adapters`
包括官方账号适配器与中转站适配器，统一封装认证、余额、模型能力、usage 抽取和错误归一化。

6. `Persistence`
SQLite 保存非敏感状态，Windows Credential Manager 保存敏感凭据。

## 5. 核心运行模型

### 5.1 ProviderEndpoint

`ProviderEndpoint` 表示一个可出流量的节点，统一抽象官方账号和第三方中转。

公共字段建议包括：

- `id`
- `name`
- `kind`，取值为 `official_account` 或 `relay_endpoint`
- `enabled`
- `priority`
- `pool_tags`
- `health_status`
- `last_health_check_at`
- `supports_balance_query`
- `last_balance_snapshot_at`
- `pricing_profile_id`
- `feature_capabilities`

官方账号附加字段：

- `auth_mode`，取值为 `imported_session` 或 `in_app_login`
- `account_identity`
- `quota_capability`
- `refresh_capability`
- `max_context_window`
- `supports_context_compression`
- `context_compression_modes`

中转附加字段：

- `relay_type`，首版至少支持 `newapi`
- `base_url`
- `model_mapping`
- `balance_capability`

### 5.2 CredentialRef

`CredentialRef` 用于引用 Windows Credential Manager 中的敏感数据。数据库只存引用，不存 secret 明文。

字段建议包括：

- `id`
- `target_name`
- `version`
- `credential_kind`
- `last_verified_at`

### 5.3 FeatureCapabilities

`FeatureCapabilities` 用于描述某个节点或某个模型可提供的官方特性能力，避免把这些能力散落在 UI 文案或请求转换逻辑里。其来源以 `CLIProxyAPI` 已实现能力为准。

字段建议包括：

- `model_id`
- `max_context_window`
- `supports_context_compression`
- `context_compression_modes`
- `supports_prompt_cache`
- `supports_reasoning`
- `supports_streaming`
- `last_capability_check_at`

设计原则：

- 能力优先按模型粒度维护，如果暂时拿不到模型级能力，可先回退到节点级默认能力
- 上下文窗口大小必须作为显式能力字段展示，而不是隐含在模型名称里
- 只有当 `CLIProxyAPI` 已实现某项官方特性时，桌面版才将其纳入能力模型
- 桌面版不实现参考项目中不存在的自研上下文压缩或其他官方特性

### 5.4 PlatformKey

`PlatformKey` 表示发给本机 Codex 客户端使用的网关 key。

字段建议包括：

- `id`
- `name`
- `key_prefix`
- `secret_ref`
- `enabled`
- `allowed_mode`，取值为 `account_only`、`relay_only`、`hybrid`
- `policy_id`
- `created_at`
- `last_used_at`

其中：

- `allowed_mode` 代表顶层许可边界
- 真正的选路细则由绑定的 `RoutingPolicy` 决定

### 5.5 RoutingPolicy

`RoutingPolicy` 是系统核心配置对象，建议独立成可复用实体，而不是把复杂规则散落到平台 key 表中。

字段建议包括：

- `id`
- `name`
- `selection_order`
- `cross_pool_fallback`
- `default_timeout_ms`
- `retry_budget`
- `failure_rules`
- `recovery_rules`
- `circuit_breaker_config`
- `request_feature_policy`

其中：

- `selection_order` 定义池顺序、标签顺序和节点分组顺序
- `failure_rules` 定义哪些失败会触发降级
- `recovery_rules` 定义熔断恢复和半开探测逻辑
- `request_feature_policy` 定义当请求声明了参考项目已支持的官方特性需求时，如何校验、降级、拒绝或改写

### 5.6 RequestLog

每次客户端请求一条主记录。

字段建议包括：

- `request_id`
- `platform_key_id`
- `request_type`
- `model`
- `selected_endpoint_id`
- `attempt_count`
- `final_status`
- `http_status`
- `started_at`
- `finished_at`
- `latency_ms`
- `error_code`
- `error_reason`
- `requested_context_window`
- `requested_context_compression`
- `effective_context_window`
- `effective_context_compression`

### 5.7 RequestAttemptLog

为了记录降级链路，每次尝试都需要单独存一条尝试记录。

字段建议包括：

- `attempt_id`
- `request_id`
- `attempt_index`
- `endpoint_id`
- `pool_type`
- `trigger_reason`
- `upstream_status`
- `timeout_ms`
- `latency_ms`
- `token_usage_snapshot`
- `estimated_cost_snapshot`
- `balance_snapshot_id`
- `feature_resolution_snapshot`

### 5.8 UsageLedger

`UsageLedger` 是 token 和费用统计的事实表。

字段建议包括：

- `usage_id`
- `request_id`
- `attempt_id`
- `platform_key_id`
- `endpoint_id`
- `model`
- `input_tokens`
- `output_tokens`
- `cache_read_tokens`
- `cache_write_tokens`
- `reasoning_tokens`
- `total_tokens`
- `estimated_cost`
- `currency`
- `usage_source`
- `price_source`
- `recorded_at`

## 6. 路由与降级规则

### 6.1 模式定义

三种模式语义固定如下：

- `account_only`：候选集只能来自官方账号池
- `relay_only`：候选集只能来自第三方中转池
- `hybrid`：候选集来自两个池，但顺序和跨池降级由策略定义

### 6.2 选路流程

每次请求的标准流程为：

1. 本机客户端携带平台 key 请求本地网关
2. 网关校验平台 key 并加载绑定策略
3. 解析请求中的官方特性需求，仅处理 `CLIProxyAPI` 已支持的特性参数
4. 根据 `allowed_mode`、`selection_order` 和特性需求生成候选队列
5. 过滤禁用、认证失效、已知额度耗尽、熔断中或能力不满足的节点
6. 按顺序尝试请求上游
7. 当命中失败规则时记录尝试日志并切换到下一个候选
8. 成功后写入请求主日志、尝试日志、usage 和健康状态

### 6.3 可配置失败条件

V1 支持策略层配置以下触发项：

- 认证失效
- 额度耗尽
- `429`
- 超时
- 连续 `5xx`
- 网络不可达

V1 不做并发探测，只做串行降级。每个请求都必须能解释“为什么选到当前节点”和“为什么放弃前一个节点”。

### 6.4 官方特性处理

V1 对官方特性的原则是：

- 仅支持 `CLIProxyAPI` 已有实现的官方特性
- 桌面版不单独新增官方特性协议或自研特性算法
- 已有实现的官方特性可以被发现、展示、透传、记录并参与选路

处理原则如下：

- 如果客户端请求显式声明了特性需求，路由器优先选择满足该能力的候选
- 若策略允许降级，可退回到能力较低但仍可执行的候选，并在日志中记录能力降级
- 若策略不允许降级，则直接返回能力不满足错误，而不是静默修改请求
- 如果某个官方节点支持参考项目已覆盖的压缩或 compact 类能力，网关应尽量透传该参数，而不是在本地伪造行为
- 本地网关首版不实现参考项目中不存在的上下文压缩算法或其他官方特性算法

实现参考优先级：

- 优先复用 `CLIProxyAPI` 里“模型注册能力约束”的思路来管理上下文窗口和最大 token 边界
- 优先复用 `CLIProxyAPI` 里 `compact` 路径透传的思路来承接官方压缩类特性
- 如果参考项目没有实现某项官方特性，则桌面版 V1 也不实现该特性

### 6.5 熔断与恢复

路由器需要支持健康状态机，至少包含：

- `healthy`
- `degraded`
- `open_circuit`
- `half_open`
- `disabled`

恢复逻辑支持：

- 熔断时长
- 半开探测
- 探测成功恢复
- 探测失败继续熔断

## 7. 官方账号管理

### 7.1 接入方式

V1 同时支持两种接入：

- 导入现有登录态
- 应用内登录

两者进入系统后统一归一化为 `OfficialSession` 运行时模型，便于复用验证、续期、余额和路由逻辑。

### 7.2 OfficialSession 运行时模型

字段建议包括：

- `session_id`
- `account_identity`
- `token_bundle_ref`
- `expires_at`
- `refresh_capability`
- `quota_capability`
- `last_verified_at`
- `status`
- `default_feature_capabilities`

### 7.3 能力探测

导入或登录成功后立即执行一次能力探测：

- 是否可用
- 是否可刷新
- 是否可查询真实额度
- 支持哪些模型或接口族
- 参考项目已支持的特性能力，例如上下文窗口大小或 `compact` 能力

对于不能查询真实额度的官方账号，V1 明确展示“余额不可查询”，不做虚假估算。

## 8. 第三方中转管理

### 8.1 适配器模型

中转站余额和调用细节不能写死在主流程中，需要定义 `RelayBalanceAdapter` 和 `RelayInvocationAdapter`。

每种中转类型至少提供：

- `supports_balance_query()`
- `query_balance()`
- `normalize_balance_response()`
- `query_models_if_supported()`
- `extract_usage()`
- `normalize_error()`

### 8.2 首版支持范围

V1 先支持：

- `newapi`
- `generic_openai_compatible_no_balance`

第二类作为兜底类型，允许调用，但明确显示“不支持余额查询”。

## 9. 余额、统计与费用口径

### 9.1 官方账号余额

口径固定为：

- 只展示真实可查询值
- 查询不到则显示“不可查询”
- 不提供本地估算额度来冒充真实额度

### 9.2 中转余额

口径固定为：

- 仅对已适配且支持余额接口的中转展示真实余额
- 其余中转明确标记“不支持查询”

### 9.3 Token 统计

优先记录上游真实 usage。若拿不到真实 usage，则在统计中明确标注来源为未知或局部估算。

标准统计字段包括：

- 输入 token
- 输出 token
- cache read token
- cache write token
- reasoning token
- total token

如果上游返回与参考项目已支持特性相关的 usage 或裁剪信息，也应归一化写入请求尝试快照或 usage 扩展字段，供后续 UI 展示。

### 9.4 费用估算

费用估算通过 `PricingProfile` 完成：

- 官方账号优先使用本地价格表估算
- 中转优先使用该中转的价格表或用户配置倍率
- 费用记录始终标记为 `estimated`

`PricingProfile` 至少包含：

- 模型匹配规则
- 输入单价
- 输出单价
- 缓存命中单价
- 币种
- 生效时间

## 10. 凭据与存储

### 10.1 安全存储

敏感信息存放于 Windows Credential Manager，包括：

- 官方账号 token 或 session
- 第三方中转 API key
- 平台 key secret

SQLite 只存：

- 配置
- 引用关系
- 日志
- 统计
- 余额快照
- 健康状态

### 10.2 持久层原则

- 前端不直接读写敏感明文
- 网关运行时通过 `CredentialRef` 读取需要的 secret
- 所有涉及请求日志和 usage 的写入使用事务，保证请求主表、尝试表和统计表一致

## 11. 管理接口与本地网关接口

### 11.1 管理接口

桌面前端通过 Tauri commands 与后端交互，执行：

- 账号登录和导入
- 中转新增与测试
- 平台 key 创建与禁用
- 策略编辑
- 余额刷新
- 状态查询
- 日志聚合查询

### 11.2 网关接口

本地 HTTP 网关只开放给本机客户端使用，建议至少包含：

- Codex 主请求入口
- `GET /health`
- `GET /models`，如果后续需要模型枚举映射

网关在请求入口需要具备一层轻量的特性解析与能力匹配逻辑：

- 识别客户端声明的目标模型
- 识别客户端声明的参考项目已支持的特性需求
- 将这些需求映射到候选节点能力矩阵
- 在无法满足时返回明确的能力不匹配错误

其中，若官方特性在参考项目中已经存在稳定入口，例如 `compact` 路径或模型能力注册逻辑，桌面网关优先保持兼容；若参考项目没有实现，则 V1 不纳入范围。

控制面动作和数据面动作必须隔离，避免在 UI 中的测试连接、余额查询等操作污染真实请求日志和用量统计。

## 12. Desktop UI 与托盘

### 12.1 主界面结构

V1 主界面建议固定为六个主区：

1. 概览
2. 官方账号
3. 第三方中转
4. 平台 Key
5. 策略中心
6. 请求日志与统计

主界面还需要有明确的能力展示区域，至少在以下位置可见：

- 官方账号详情页展示参考项目已支持的能力情况，例如上下文窗口大小或 `compact` 能力
- 模型或能力详情弹层展示模型级能力矩阵
- 请求日志详情展示请求声明能力与实际生效能力

### 12.2 首启默认对象

首次启动自动创建：

- 一个启用中的 `default key`
- 一个系统内置 `default policy`

默认行为：

- `default key.allowed_mode = hybrid`
- `default key.policy_id = default policy`

如果用户删除 `default key`，系统不自动重建。若只是禁用，则托盘快捷切换功能变为只读提示。

### 12.3 Tray 右键面板

Tauri v2 托盘面板定位为增强运维面板，不承载复杂编辑。

建议菜单项包括：

- 网关状态
- 当前监听地址
- `default key` 当前模式
- 切换到 `account_only`
- 切换到 `relay_only`
- 切换到 `hybrid`
- 当前可用官方账号数
- 当前可用中转数
- 最近一次余额刷新摘要
- 打开主界面
- 重启网关
- 退出

行为原则：

- 托盘只操作 `default key`
- 托盘不直接编辑复杂策略
- 左键单击托盘图标唤起主窗口
- 右键打开菜单
- 当当前模式下没有可用节点时，允许切换，但需明确显示该模式无可用节点

## 13. 错误模型

V1 将错误统一收敛为以下类别：

- `CredentialError`
- `QuotaError`
- `RoutingError`
- `UpstreamError`
- `ConfigError`

UI 展示面向用户的动作性错误文案，不直接展示内部堆栈。

示例：

- 该官方账号登录态已过期，请重新登录
- 该中转类型当前不支持余额查询
- 此平台 key 的策略未包含任何可用节点
- 请求已降级多次，所有候选节点均失败
- 请求要求的官方特性当前无可用节点支持，或该特性不在 V1 支持范围内

## 14. 测试策略

V1 测试重点放在后端稳定性，而不是完整 UI 自动化。

### 14.1 单元测试

- 路由器优先级和模式筛选
- 失败规则触发
- 熔断和恢复
- 官方账号能力探测
- 参考项目已支持的官方特性能力匹配
- `compact` 或类似已实现特性的透传与拒绝逻辑
- NewAPI 余额归一化
- usage 抽取与价格表匹配

### 14.2 持久层测试

- 请求主表、尝试表、统计表事务一致性
- 平台 key secret 不落库
- 凭据引用有效性

### 14.3 集成测试

通过本地测试 server 模拟：

- 成功请求
- `429`
- 超时
- 连续 `5xx`
- 额度耗尽

验证：

- 是否正确降级
- 是否正确写入日志
- 是否正确更新健康状态

### 14.4 Tauri 后端测试

验证 commands：

- 创建和读取平台 key
- 登录态导入与校验
- 中转连接测试
- 余额刷新
- 日志查询

## 15. 分阶段实施建议

推荐实现顺序：

1. 项目骨架与基础运行时
2. SQLite 与凭据存储封装
3. 平台 key 和策略模型
4. 本地 loopback gateway
5. 路由引擎和日志链路
6. 官方账号适配器
7. NewAPI 中转适配器
8. 概览页和列表管理页
9. 托盘与 `default key`
10. 统计与余额面板

这样可以优先建立最关键的控制面和数据面闭环，再补充 UI 完整度。

## 16. 成功标准

当以下条件满足时，V1 设计可进入实现计划阶段：

- Windows 上可启动桌面应用和本地 loopback gateway
- 首启自动生成 `default key`
- 本机客户端可使用平台 key 访问本地网关
- 官方账号和 NewAPI 中转都能被纳入统一选路
- 参考项目已支持的官方特性能力可以被发现、展示、记录并参与选路
- 降级规则可配置且有可解释日志
- 敏感凭据不落库明文
- 托盘可切换 `default key` 三种模式并显示摘要状态
- 请求日志、token 统计和费用估算可按 key 与节点维度查看
