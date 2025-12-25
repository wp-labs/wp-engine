pub mod error;
// core 子模块默认收紧为 crate 内可见；外部通过 facade 暴露最小集
pub(crate) mod generator;
pub(crate) mod parser;
mod prelude;
pub(crate) mod sinks;
// 已移除对第三方 Source API 的 re-export；内部直接用 wp_connector_api::runtime::source
// mod sources; // PR3: 停止编译该过渡模块

// Core 内部通道：将广域 re-export 收紧为 crate 级别；
// 外部请通过 facade 或直接依赖对应 API crate。
#[allow(unused_imports)]
pub(crate) use crate::core::error::strategies::ErrReport;
#[allow(unused_imports)]
pub(crate) use crate::core::error::strategies::current_error_policy;
#[allow(unused_imports)]
pub(crate) use crate::core::generator::rules::GenRuleUnit;
#[allow(unused_imports)]
pub(crate) use parser::WplPipeline;
#[allow(unused_imports)]
pub(crate) use parser::WplRepository;
#[allow(unused_imports)]
pub(crate) use sinks::async_sink::AsyncCtrl;
#[allow(unused_imports)]
pub(crate) use sinks::async_sink::AsyncRawdatSink;
#[allow(unused_imports)]
pub(crate) use sinks::async_sink::AsyncRecordSink;
#[allow(unused_imports)]
pub(crate) use sinks::sync_sink::{RecSyncSink, SinkTerminal, SyncCtrl, TrySendStatus};
