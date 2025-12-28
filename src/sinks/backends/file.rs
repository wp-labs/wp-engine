use crate::core::sinks::sync_sink::RecSyncSink;
use crate::core::{SyncCtrl, TrySendStatus};
use crate::sinks::prelude::*;
use crate::sinks::utils::buffer_monitor::BufferMonitor;
use crate::sinks::utils::formatter::FormatAdapter;
use crate::sinks::{SinkEndpoint, SinkRecUnit};
use crate::types::{AnyResult, Build1, SafeH};
use anyhow::Context;
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;
use wp_data_fmt::{DataFormat, FormatType};
// use orion_conf::ToStructError; // unused here
use orion_error::ErrorOwe;
use std::fs;
use std::fs::File;
use std::io::{Cursor, Write};
use tokio::fs::OpenOptions;
use tokio::io::{AsyncWriteExt, BufWriter};
// no mpsc/interval after fast_file pivot
use tokio_async_drop::tokio_async_drop;
use wp_connector_api::{SinkBuildCtx, SinkReason, SinkResult, SinkSpec as ResolvedSinkSpec};
use wp_model_core::model::fmt_def::TextFmt;

pub fn create_watch_out(fmt: TextFmt) -> (SafeH<Cursor<Vec<u8>>>, SinkEndpoint) {
    let buffer_out = BufferMonitor::new();
    let buffer = buffer_out.buffer.clone();
    let mut out: FormatAdapter<BufferMonitor> = FormatAdapter::new(fmt);
    out.next_pipe(buffer_out);
    let out = SinkEndpoint::Buffer(out);
    (buffer, out)
}

#[derive(Clone, Debug)]
pub(crate) struct FileSinkSpec {
    fmt: TextFmt,
    base: String,
    file_name: String,
    path: String,
    use_base_path: bool,
}

impl FileSinkSpec {
    pub(crate) fn from_resolved(kind: &str, spec: &ResolvedSinkSpec) -> AnyResult<Self> {
        let has_path = spec.params.get("path").and_then(|v| v.as_str()).is_some();
        let has_base_file = spec.params.get("base").and_then(|v| v.as_str()).is_some()
            && spec.params.get("file").and_then(|v| v.as_str()).is_some();
        if !(has_path || has_base_file) {
            anyhow::bail!("{} sink requires either 'path' or 'base'+'file'", kind);
        }
        if let Some(s) = spec.params.get("fmt").and_then(|v| v.as_str()) {
            let ok = matches!(
                s,
                "json" | "csv" | "show" | "kv" | "raw" | "proto" | "proto-text"
            );
            if !ok {
                anyhow::bail!(
                    "invalid fmt: '{}'; allowed: json,csv,show,kv,raw,proto,proto-text",
                    s
                );
            }
        }
        let fmt = spec
            .params
            .get("fmt")
            .and_then(|v| v.as_str())
            .map(TextFmt::from)
            .unwrap_or(TextFmt::Json);
        let base = spec
            .params
            .get("base")
            .and_then(|v| v.as_str())
            .unwrap_or("./data/out_dat")
            .to_string();
        let file_name = spec
            .params
            .get("file")
            .and_then(|v| v.as_str())
            .unwrap_or("out.dat")
            .to_string();
        let path = spec
            .params
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("./data/out_dat/out.dat")
            .to_string();
        Ok(Self {
            fmt,
            base,
            file_name,
            path,
            use_base_path: has_base_file,
        })
    }

    pub(crate) fn text_fmt(&self) -> TextFmt {
        self.fmt
    }

    pub(crate) fn resolve_path(&self, _ctx: &SinkBuildCtx) -> String {
        if self.use_base_path {
            Path::new(&self.base)
                .join(&self.file_name)
                .display()
                .to_string()
        } else {
            self.path.clone()
        }
    }
}

#[derive(Clone)]
pub struct FileSink {
    path: String,
    out_io: SafeH<std::fs::File>,
    buffer: Cursor<Vec<u8>>,
}

impl FileSink {
    pub fn new(out_path: &str) -> AnyResult<Self> {
        let out_io =
            File::create(out_path).with_context(|| format!("output file fail :{}", out_path))?;
        Ok(Self {
            path: out_path.to_string(),
            out_io: SafeH::build(out_io),
            buffer: Cursor::new(Vec::with_capacity(10240)),
        })
    }
}

impl Drop for FileSink {
    fn drop(&mut self) {
        if let Some(new_path) = self.path.strip_suffix(".lock")
            && let Err(e) = fs::rename(&self.path, new_path)
        {
            error_data!("解锁备份文件失败,{}", e);
        }
    }
}

impl SyncCtrl for FileSink {
    fn stop(&mut self) -> SinkResult<()> {
        if let Ok(mut out_io) = self.out_io.write() {
            out_io
                .write_all(&self.buffer.clone().into_inner())
                .owe(SinkReason::Sink("file stop fail".into()))?;
        }
        // Rename .lock -> .dat on explicit stop to ensure unlock even if Drop timing varies
        if let Some(new_path) = self.path.strip_suffix(".lock")
            && let Err(e) = fs::rename(&self.path, new_path)
        {
            error_data!("unlock rescue file on stop failed: {}", e);
        }
        Ok(())
    }
}

#[async_trait]
impl RecSyncSink for FileSink {
    fn send_to_sink(&self, data: SinkRecUnit) -> SinkResult<()> {
        // FileSink 处理记录数据，将其格式化为字符串后写入
        // 这里应该根据配置的格式进行转换
        // 暂时实现为简单的格式输出
        if let Ok(mut out_io) = self.out_io.write() {
            // 使用默认的格式化输出
            let formatted = FormatType::from(&wp_model_core::model::fmt_def::TextFmt::Raw)
                .format_record(data.data());
            out_io
                .write_all(format!("{}\n", formatted).as_bytes())
                .owe(SinkReason::sink("file out fail"))?;
        }
        Ok(())
    }

    fn try_send_to_sink(&self, data: SinkRecUnit) -> TrySendStatus {
        match self.send_to_sink(data) {
            Ok(()) => TrySendStatus::Sended,
            Err(e) => TrySendStatus::Err(Arc::new(e)),
        }
    }
}

/*
#[async_trait]
impl FFVSyncSink for FileSink {
    fn send_ffv_to_sink(&self, data: SinkFFVUnit) -> SinkResult<()> {
        // FileSink 可以处理 FFV 数据，将其转换为字符串写入
        // 这里应该根据配置的格式进行转换
        // 暂时实现为简单的字符串输出
        if let Ok(mut out_io) = self.out_io.write() {
            // 简单地将 FFV 数据转换为字符串输出
            // 实际实现应该根据格式配置进行转换
            for field in data.data() {
                out_io
                    .write_all(format!("{}\t", field.data_field).as_bytes())
                    .owe(SinkReason::sink("file out fail"))?;
            }
            out_io
                .write_all(b"\n")
                .owe(SinkReason::sink("file out fail"))?;
        }
        Ok(())
    }

    fn try_send_ffv_to_sink(&self, data: SinkFFVUnit) -> TrySendStatus {
        match self.send_ffv_to_sink(data) {
            Ok(()) => TrySendStatus::Sended,
            Err(e) => TrySendStatus::Err(Arc::new(e)),
        }
    }

    fn send_ffv_batch_to_sink(&self, data: Vec<SinkFFVUnit>) -> SinkResult<()> {
        for ffv_unit in data {
            self.send_ffv_to_sink(ffv_unit)?;
        }
        Ok(())
    }

    fn try_send_ffv_batch_to_sink(&self, data: Vec<SinkFFVUnit>) -> Vec<TrySendStatus> {
        data.into_iter()
            .map(|unit| self.try_send_ffv_to_sink(unit))
            .collect()
    }
}
*/

// Default buffer for classic file sink (kept for compatibility)
const FILE_BUF_SIZE: usize = 102_400; // 100 KiB

// fast_file 环境变量解析已移除

// Classic async file sink (original behavior preserved):
// - Direct BufWriter writes
// - Periodic flush by count (every 100 writes)
pub struct AsyncFileSink {
    path: String,
    out_io: BufWriter<tokio::fs::File>,
    proc_cnt: usize,
}

impl Drop for AsyncFileSink {
    fn drop(&mut self) {
        if let Some(new_path) = self.path.strip_suffix(".lock")
            && let Err(e) = fs::rename(&self.path, new_path)
        {
            error_data!("解锁备份文件失败,{}", e);
        }
        tokio_async_drop!({
            let _ = self.out_io.flush().await;
        });
    }
}

impl AsyncFileSink {
    pub async fn new(out_path: &str) -> AnyResult<Self> {
        //crate dir if  path  parent  not exist
        if let Some(parent) = std::path::Path::new(out_path).parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)?;
        }
        let out_io = OpenOptions::new()
            .append(true)
            .create(true)
            .open(out_path)
            .await
            .with_context(|| format!("output file fail :{}", out_path))?;
        Ok(Self {
            path: out_path.to_string(),
            out_io: BufWriter::with_capacity(FILE_BUF_SIZE, out_io),
            proc_cnt: 0,
        })
    }
}

#[async_trait]
impl AsyncCtrl for AsyncFileSink {
    async fn stop(&mut self) -> SinkResult<()> {
        self.out_io
            .flush()
            .await
            .owe(SinkReason::sink("file out fail"))?;
        // Rename .lock -> .dat on explicit stop to make unlock robust on graceful shutdown
        if let Some(new_path) = self.path.strip_suffix(".lock")
            && let Err(e) = fs::rename(&self.path, new_path)
        {
            error_data!("unlock rescue file on stop failed: {}", e);
        }
        Ok(())
    }

    async fn reconnect(&mut self) -> SinkResult<()> {
        Ok(())
    }
}

#[async_trait]
impl AsyncRawdatSink for AsyncFileSink {
    async fn sink_bytes(&mut self, data: &[u8]) -> SinkResult<()> {
        self.out_io
            .write_all(data)
            .await
            .owe(SinkReason::sink("file out fail"))?;
        self.proc_cnt += 1;
        if self.proc_cnt.is_multiple_of(100) {
            self.out_io
                .flush()
                .await
                .owe(SinkReason::sink("file out fail"))?;
        }
        Ok(())
    }
    async fn sink_str(&mut self, data: &str) -> SinkResult<()> {
        if data.as_bytes().last() == Some(&b'\n') {
            self.sink_bytes(data.as_bytes()).await
        } else {
            self.out_io
                .write_all(data.as_bytes())
                .await
                .owe(SinkReason::sink("file out fail"))?;
            self.out_io
                .write_all(b"\n")
                .await
                .owe(SinkReason::sink("file out fail"))?;
            Ok(())
        }
    }

    async fn sink_str_batch(&mut self, data: Vec<&str>) -> SinkResult<()> {
        if data.is_empty() {
            return Ok(());
        }

        // 计算总长度，预分配缓冲区
        let mut total_len = 0;
        for str_data in &data {
            total_len += str_data.len();
            // 如果字符串没有换行符，需要添加一个
            if str_data.as_bytes().last().is_none_or(|&b| b != b'\n') {
                total_len += 1;
            }
        }

        // 合并所有数据到单个缓冲区
        let mut buffer = Vec::with_capacity(total_len);
        for str_data in &data {
            buffer.extend_from_slice(str_data.as_bytes());
            if str_data.as_bytes().last().is_none_or(|&b| b != b'\n') {
                buffer.push(b'\n');
            }
        }

        // 一次性写入所有数据
        self.out_io
            .write_all(&buffer)
            .await
            .owe(SinkReason::sink("file out fail"))?;

        // 更新计数器并检查是否需要刷新
        self.proc_cnt += data.len();
        if self.proc_cnt.is_multiple_of(100) {
            self.out_io
                .flush()
                .await
                .owe(SinkReason::sink("file out fail"))?;
        }

        Ok(())
    }

    async fn sink_bytes_batch(&mut self, data: Vec<&[u8]>) -> SinkResult<()> {
        if data.is_empty() {
            return Ok(());
        }

        // 计算总长度，预分配缓冲区
        let mut total_len = 0;
        for bytes_data in &data {
            total_len += bytes_data.len();
            // 如果数据没有换行符，需要添加一个
            if bytes_data.last().is_none_or(|&b| b != b'\n') {
                total_len += 1;
            }
        }

        // 合并所有数据到单个缓冲区
        let mut buffer = Vec::with_capacity(total_len);
        for bytes_data in &data {
            buffer.extend_from_slice(bytes_data);
            if bytes_data.last().is_none_or(|&b| b != b'\n') {
                buffer.push(b'\n');
            }
        }

        // 一次性写入所有数据
        self.out_io
            .write_all(&buffer)
            .await
            .owe(SinkReason::sink("file out fail"))?;

        // 更新计数器并检查是否需要刷新
        self.proc_cnt += data.len();
        if self.proc_cnt.is_multiple_of(100) {
            self.out_io
                .flush()
                .await
                .owe(SinkReason::sink("file out fail"))?;
        }

        Ok(())
    }
}

// fast_file 类型已移除
// fast_file 类型已移除

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write as _;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};
    use wp_connector_api::AsyncRecordSink;
    use wp_model_core::model::DataRecord;
    use wp_model_core::model::fmt_def::TextFmt;

    use crate::sinks::backends::file::AsyncFileSink;
    use crate::sinks::utils::formatter::AsyncFormatter;
    use crate::types::AnyResult;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_processor() -> AnyResult<()> {
        let mut pipe: AsyncFormatter<AsyncFileSink> = AsyncFormatter::new(TextFmt::Json);
        pipe.next_pipe(AsyncFileSink::new("./temp/test.pid").await?);
        let test_data = DataRecord::default();
        pipe.sink_record(&test_data).await?;
        Ok(())
    }

    // 仅释放“自身创建”的 .lock：显式 stop 应将自身的 .lock -> .dat，且不会影响其它 .lock 文件
    #[tokio::test(flavor = "multi_thread")]
    async fn stop_unlocks_only_own_lock() -> AnyResult<()> {
        // 准备唯一临时目录
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let base = std::env::temp_dir().join(format!("wp_rescue_unlock_{}", ts));
        let own_lock = base.join("group1/sinkA-001.dat.lock");
        let other_lock = base.join("group1/sinkB-001.dat.lock");
        if let Some(p) = own_lock.parent() {
            fs::create_dir_all(p)?;
        }
        if let Some(p) = other_lock.parent() {
            fs::create_dir_all(p)?;
        }

        // 创建另一个 .lock（非本 sink 持有），模拟外部/他处残留
        fs::File::create(&other_lock)?.write_all(b"test")?;

        // 构建本 sink，写入并显式停止
        let mut sink = AsyncFileSink::new(own_lock.to_string_lossy().as_ref()).await?;
        // 写入一行，确保有 IO 发生
        wp_connector_api::AsyncRawDataSink::sink_str(&mut sink, "line1").await?;
        wp_connector_api::AsyncCtrl::stop(&mut sink).await?;

        // 断言：自身 .lock 已重命名为 .dat，且其它 .lock 不受影响
        assert!(!Path::new(own_lock.to_string_lossy().as_ref()).exists());
        assert!(Path::new(base.join("group1/sinkA-001.dat").to_string_lossy().as_ref()).exists());
        assert!(Path::new(other_lock.to_string_lossy().as_ref()).exists());

        // 清理
        let _ = fs::remove_dir_all(&base);
        Ok(())
    }
}
