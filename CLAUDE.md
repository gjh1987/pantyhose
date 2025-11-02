# CLAUDE.md - Pantyhose Server 代码规范和功能文档

## 项目概述

**Pantyhose Server** 是一个高性能分布式游戏服务器框架，采用 Rust 语言实现。采用**新一代 RPC 消息系统 + 无状态消息处理器架构**，支持完整的 RPC 调用链路和统一的消息类型管理。

## 核心架构约束

### ⚠️ 重要：单线程架构约束

**本服务器的主线程采用单线程设计，这是核心架构约束，请勿随意修改！**

- **主线程单线程执行**：所有业务逻辑、配置管理、错误处理都在主线程中串行执行
- **日志系统设计**：基于主线程单线程假设，使用 `tracing-appender` 的 `non_blocking` 模式将 I/O 操作委托给后台线程
- **LogGuard 生命周期**：Server 结构体中的 `_log_guard` 字段必须保持存活，用于维持后台日志线程的生命周期
- **无互斥锁设计**：由于主线程单线程特性，日志写入无需使用 Mutex 等同步原语

## 代码规范

### 1. 命名规范

#### 文件命名
- 使用蛇形命名法：`snake_case.rs`
- 模块文件：`mod.rs`
- 配置文件：`config.xml`

#### 类型命名
- 结构体：`PascalCase`，如 `ServerConfig`
- Trait：`PascalCase`，如 `ServerTrait`
- 枚举：`PascalCase`，如 `ServerType`
- 常量：`SCREAMING_SNAKE_CASE`，如 `MSG_ID_CHAT_TEST_B_REQUEST`

#### 变量命名
- 局部变量：`snake_case`，如 `server_id`
- 成员变量：`snake_case`，如 `front_session_manager`
- 私有字段：`snake_case`，如 `_log_guard`

### 2. 代码组织

#### 模块结构
```
src/
├── framework/           # 核心框架组件
├── servers/            # 具体服务器实现
├── proto/              # 协议定义
└── main.rs             # 程序入口
```

#### 导入规范
```rust
// 标准库导入
use std::env;

// 外部依赖导入
use tokio::sync::Notify;

// 内部模块导入
use crate::framework::server::ServerTrait;
use crate::servers::chat::chat_server::ChatServer;
```

### 3. 错误处理模式

#### 简化错误处理
- 函数返回 `bool` 表示成功/失败状态
- 错误详情通过 `tracing::error!` 记录到日志
- 保持代码简洁，避免复杂的错误传播链

#### ⚠️ 重要：禁止使用 `!` 操作符
- **禁止使用 `!` 操作符**：所有布尔值的否定检查必须使用 `== false`
- **提高可读性**：`== false` 比 `!` 更明确地表达意图
- **保持一致性**：整个项目使用统一的布尔检查方式

```rust
// ✅ 正确：使用 == false
pub fn init(&mut self, config: &Config) -> bool {
    if self.setup_network() == false {
        tracing::error!("Failed to setup network");
        return false;
    }
    true
}

// ❌ 错误：禁止使用 ! 操作符
pub fn init(&mut self, config: &Config) -> bool {
    if !self.setup_network() {  // 禁止使用 !
        tracing::error!("Failed to setup network");
        return false;
    }
    true
}
```

### 4. 日志规范

#### 日志级别使用
- `tracing::debug!` - 调试信息
- `tracing::info!` - 常规信息
- `tracing::warn!` - 警告信息
- `tracing::error!` - 错误信息

#### ⚠️ 重要：AI 生成的日志限制
- **AI 只能添加错误日志**：AI 生成的代码只能使用 `tracing::error!` 记录错误信息
- **禁止重复错误日志**：相同的错误信息不能在代码中重复记录
- **调试信息由开发者添加**：所有 `tracing::debug!` 和 `tracing::info!` 日志必须由开发者手动添加

#### 日志格式
```rust
// ✅ AI 可以添加的错误日志
tracing::error!("Failed to connect to cluster: {}", error);

// ❌ AI 禁止添加的调试信息（必须由开发者手动添加）
tracing::debug!("Processing message: {:?}", message);
tracing::info!("Server {} initialized successfully", server_id);
```

### 5. 注释规范

#### 文档注释
```rust
/// RPC管理器，负责管理RPC调用和路由
///
/// ## 重要说明
/// 本类为单例，被Server持有，所有方法都在主线程调用，不存在线程安全问题。
pub struct RpcManager {
    // ...
}
```

#### 内联注释
```rust
// 保存管理器指针 - 注意：主线程单线程安全
self.front_session_manager = front_session_manager as *mut FrontSessionManager;
```

## 核心功能特性

### 1. 单线程异步架构

#### 运行时配置
```rust
#[tokio::main]
async fn main() {
    // 主线程单线程执行
}
```

#### 生命周期管理
```rust
// 服务器生命周期必须按顺序执行
server.init(server_id, &config);
server.late_init();
server.run().await;
server.dispose();
```

### 2. RPC 消息系统

#### 消息类型一致性
- **网络协议层**：使用 u32 (Protobuf 定义)
- **内部处理层**：使用 u32 (避免类型转换)
- **存储优化层**：使用 u16 (HashMap 键类型，节省内存)

#### RPC 调用流程
```
客户端 → FrontSession → ForwardManager → BackSession → RpcMessageDispatcher → 业务处理器
```

### 3. 无状态消息处理器架构

#### MsgProcessor Trait
```rust
pub trait MsgProcessor {
    fn process_message(&self, session: &mut dyn Session, message: &dyn Any) -> bool;
}
```

#### 共享引用模式
```rust
// 使用 Arc<dyn MsgProcessor> 实现多点共享，无需 Mutex
Server::msg_processor: Arc<dyn MsgProcessor>
    ↓ (共享引用)
SessionManager::msg_processor: Option<Arc<dyn MsgProcessor>>
    ↓ (在创建session时)
Connection::msg_processor: Option<Arc<dyn MsgProcessor>>
```

### 4. 网络架构

#### 多协议支持
- **TCP 连接**：`TcpConnection`
- **WebSocket 连接**：`WebSocketConnection`
- **前端/后端分离**：不同端口用于客户端 vs 服务器通信

#### 连接管理
```rust
// 前端会话管理
FrontSessionManager
// 后端会话管理
BackSessionManager
// 网络引擎
NetworkEngine
```

### 5. 集群支持

#### 集群组件
- `ClusterManager` - 集群管理器
- `ServerManager` - 服务器管理器
- `MasterServer` - 主控服务器

#### 服务发现
- 动态服务器注册和发现
- 基于服务器类型的路由
- 集群状态监控

## 开发指南

### 1. 新增服务器类型

#### 步骤 1：创建服务器实现
```rust
pub struct MyServer {
    // 服务器特定字段
}

impl ServerTrait for MyServer {
    fn init(&mut self, server_id: u32, config: &Config) -> bool {
        // 初始化逻辑
        true
    }

    // 实现其他方法
}
```

#### 步骤 2：注册到主程序
```rust
// 在 main.rs 中添加
let mut server = if server_group.name == MyServer::ServerName() {
    ServerType::My(MyServer::new())
} else if // ... 其他服务器类型
```

#### 步骤 3：添加路由函数
```rust
// 注册路由函数
rpc_manager.add_router(MyServer::ServerType(), Box::new(my_router_function));
```

### 2. 添加 Protocol Buffer 消息

#### 步骤 1：创建 .proto 文件
```protobuf
// 在 tools/proto/config/ 目录创建
syntax = "proto3";

package mypackage;

message MyBRequest {
    uint32 msg_unique_id = 1;
    string data = 2;
}

message MyBResponse {
    uint32 msg_unique_id = 1;
    int32 result = 2;
}
```

#### 步骤 2：生成消息代码
```bash
# 运行生成脚本
tools/shell/generate_protobuf_message_id.cmd
```

#### 步骤 3：实现消息处理器
```rust
impl MyMessageHandler {
    pub fn init(&mut self, dispatcher: &mut RpcMessageDispatcher, rpc_manager: &mut RpcManager) {
        dispatcher.register_request_handler(
            MSG_ID_MY_B_REQUEST,
            Box::new(|session, msg_unique_id, front_session_id, msg_id, message| {
                Self::handle_my_request_static(session, msg_unique_id, front_session_id, msg_id, message);
            }),
        );
    }
}
```

### 3. 配置管理

#### XML 配置文件结构
```xml
<?xml version="1.0" encoding="UTF-8"?>
<config>
    <servers>
        <group name="master">
            <server id="1" back_tcp_port="3000"/>
        </group>
        <group name="chat">
            <server id="11" back_tcp_port="3101" front_tcp_port="3001" front_ws_port="3011"/>
        </group>
    </servers>

    <log debug="terminal|file"
         info="terminal|file"
         net="terminal|file"
         warn="terminal|file"
         err="terminal|file"/>

    <author key="your_auth_key"/>
    <run_time worker_threads="4"/>
</config>
```

#### 配置项说明
- `back_tcp_port` - 后端服务器间通信端口
- `front_tcp_port` - 前端 TCP 客户端连接端口
- `front_ws_port` - 前端 WebSocket 客户端连接端口
- `worker_threads` - Tokio 运行时的工作线程数

## 性能优化

### 1. 单线程优势
- 避免锁竞争，提高 CPU 缓存命中率
- 简化内存模型，减少内存屏障开销
- 便于性能分析和调试

### 2. 异步 I/O 优化
- 使用 Tokio 的异步运行时
- 批量处理网络消息
- 非阻塞日志写入

### 3. 内存管理
- 使用 Arc 进行引用计数
- 避免频繁的内存分配
- 复用缓冲区

## 调试和测试

### 1. 调试工具

#### Tauri 调试工具
- **实时消息监控**：支持 TCP 和 WebSocket 协议
- **RPC 消息解析**：自动解析 RPC 响应内部消息
- **协议管理**：动态加载 .proto 文件
- **消息模板**：快速发送测试消息

#### 日志调试
```rust
// 启用详细日志
tracing::debug!("Processing message: {:?}", message);
tracing::info!("Session created: {}", session_id);
```

### 2. 测试策略

#### 单元测试
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_processing() {
        // 测试逻辑
    }
}
```

#### 集成测试
- 测试服务器间通信
- 验证 RPC 调用链路
- 检查集群协调功能

## 部署和运维

### 1. 编译和运行

#### 编译项目
```bash
cargo build
```

#### 运行服务器
```bash
# 默认配置
cargo run

# 指定配置文件和服务器ID
cargo run -- bin/config.xml 11
```

### 2. 监控和日志

#### 日志配置
- 终端输出：`terminal`
- 文件输出：`file`
- 可同时启用：`terminal|file`

#### 性能监控
- 使用 `tracing` 框架记录性能指标
- 监控网络连接状态
- 跟踪 RPC 调用延迟

## 安全注意事项

### 1. 认证和授权
- 使用配置中的 `author key` 进行服务器间认证
- 验证客户端连接令牌
- 实现基于角色的访问控制

### 2. 输入验证
- 验证 Protocol Buffer 消息格式
- 检查消息大小限制
- 防范缓冲区溢出攻击

### 3. 网络安全
- 使用 TLS 加密通信（如需要）
- 实现连接速率限制
- 监控异常连接行为

## 扩展和定制

### 1. 自定义路由逻辑
```rust
pub type RouterFunction = Box<dyn Fn(&FrontSessionMetaData) -> Option<u32>>;

// 注册自定义路由函数
rpc_manager.add_router("my_server_type", Box::new(my_custom_router));
```

### 2. 自定义消息处理器
```rust
// 实现自定义消息处理器
trait CustomMessageHandler {
    fn handle_custom_message(&self, session: &mut BackSession, message: &dyn Any) -> bool;
}
```

### 3. 插件系统
- 支持动态加载消息处理器
- 可扩展的服务器类型注册
- 模块化配置管理

## 故障排除

### 1. 常见问题

#### 日志后台线程终止
- **症状**：日志停止输出
- **原因**：`LogGuard` 被提前释放
- **解决**：确保 `Server._log_guard` 字段在整个服务器生命周期保持存活

#### RPC 调用超时
- **症状**：RPC 响应未返回
- **原因**：消息路由错误或处理器未注册
- **解决**：检查消息 ID 映射和处理器注册

#### 连接断开
- **症状**：客户端连接频繁断开
- **原因**：网络配置错误或防火墙问题
- **解决**：验证端口配置和网络连通性

#### 代码规范检查
- **症状**：代码审查不通过或编译警告
- **原因**：使用了禁止的 `!` 操作符
- **解决**：将所有 `!` 操作符替换为 `== false`

### 2. 调试技巧

#### 启用详细日志
```rust
// 在配置中启用所有日志级别
<log debug="terminal|file"
     info="terminal|file"
     net="terminal|file"
     warn="terminal|file"
     err="terminal|file"/>
```

#### 使用调试工具
- 运行 Tauri 调试工具监控消息流
- 检查 RPC 消息的完整调用链路
- 验证 Protocol Buffer 消息格式

---

**注意**：本项目的核心架构约束是单线程设计，任何修改都必须遵守这一原则。在扩展功能时，优先使用现有的框架组件和模式，避免引入破坏性变更。

## 代码规范强制要求

### ⚠️ 禁止使用 `!` 操作符

**所有布尔值的否定检查必须使用 `== false`，禁止使用 `!` 操作符。**

#### 示例
```rust
// ✅ 正确
if condition == false {
    // 处理条件为假的情况
}

// ❌ 错误
if !condition {
    // 禁止使用 ! 操作符
}
```

#### 适用场景
- 函数返回值检查：`if function() == false`
- 布尔变量检查：`if is_connected == false`
- 方法调用检查：`if object.method() == false`
- 表达式检查：`if (a && b) == false`

#### 原因
- **提高可读性**：`== false` 比 `!` 更明确地表达意图
- **减少歧义**：避免 `!` 操作符可能带来的理解困难
- **保持一致性**：整个项目使用统一的布尔检查方式

**违反此规范将导致代码审查不通过。**

### ⚠️ AI 生成的日志限制

**AI 生成的代码只能添加错误日志，且不能重复记录相同的错误信息。所有调试信息必须由开发者手动添加。**

#### 允许的 AI 日志
```rust
// ✅ AI 可以添加的错误日志
tracing::error!("Failed to initialize database connection");
tracing::error!("RPC call failed with error: {}", error);
tracing::error!("Session {} disconnected unexpectedly", session_id);
```

#### 禁止的 AI 日志
```rust
// ❌ AI 禁止添加的调试信息
tracing::debug!("Processing message ID: {}", msg_id);
tracing::info!("Server started successfully");
tracing::info!("Connected to database: {}", db_name);
```

#### 原因
- **错误日志是必要的**：错误信息对于系统稳定性和问题排查至关重要
- **避免日志污染**：过多的调试信息会干扰开发者对重要信息的关注
- **保持日志质量**：由开发者手动添加的调试信息更有针对性和价值

**违反此限制的代码将需要重新审查和修改。**