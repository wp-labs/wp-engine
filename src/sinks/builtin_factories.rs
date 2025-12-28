use crate::sinks::backends::file::FileSinkSpec;
use crate::sinks::backends::tcp::TcpFactory;
use crate::sinks::sink_build::build_file_sink;
use crate::sinks::{ASinkTestProxy, BlackHoleSink, HealthController, SyslogFactory};
use async_trait::async_trait;
//

// Built-in lightweight no-op sink implementing Async* traits
//pub struct BlackHoleSink;

struct BlackHoleFactory;

#[async_trait]
impl wp_connector_api::SinkFactory for BlackHoleFactory {
    fn kind(&self) -> &'static str {
        "blackhole"
    }
    fn validate_spec(&self, _spec: &wp_connector_api::SinkSpec) -> anyhow::Result<()> {
        // no params required
        Ok(())
    }
    async fn build(
        &self,
        _spec: &wp_connector_api::SinkSpec,
        _ctx: &wp_connector_api::SinkBuildCtx,
    ) -> anyhow::Result<wp_connector_api::SinkHandle> {
        Ok(wp_connector_api::SinkHandle::new(Box::new(
            BlackHoleSink {},
        )))
    }
}

struct FileFactory;

#[async_trait]
impl wp_connector_api::SinkFactory for FileFactory {
    fn kind(&self) -> &'static str {
        "file"
    }
    fn validate_spec(&self, spec: &wp_connector_api::SinkSpec) -> anyhow::Result<()> {
        FileSinkSpec::from_resolved("file", spec).map(|_| ())
    }
    async fn build(
        &self,
        spec: &wp_connector_api::SinkSpec,
        ctx: &wp_connector_api::SinkBuildCtx,
    ) -> anyhow::Result<wp_connector_api::SinkHandle> {
        let resolved = FileSinkSpec::from_resolved("file", spec)?;
        let path = resolved.resolve_path(ctx);
        let fmt = resolved.text_fmt();
        let dummy = wp_conf::structure::SinkInstanceConf::null_new(spec.name.clone(), fmt, None);
        // Build using existing file builder (AsyncFormatter<AsyncFileSink>)
        let f = build_file_sink(&dummy, &path).await?;
        Ok(wp_connector_api::SinkHandle::new(Box::new(f)))
    }
}

struct TestRescueFactory;

#[async_trait]
impl wp_connector_api::SinkFactory for TestRescueFactory {
    fn kind(&self) -> &'static str {
        "test_rescue"
    }
    fn validate_spec(&self, spec: &wp_connector_api::SinkSpec) -> anyhow::Result<()> {
        FileSinkSpec::from_resolved("test_rescue", spec).map(|_| ())
    }
    async fn build(
        &self,
        spec: &wp_connector_api::SinkSpec,
        ctx: &wp_connector_api::SinkBuildCtx,
    ) -> anyhow::Result<wp_connector_api::SinkHandle> {
        let resolved = FileSinkSpec::from_resolved("test_rescue", spec)?;
        let path = resolved.resolve_path(ctx);
        let fmt = resolved.text_fmt();
        let dummy = wp_conf::structure::SinkInstanceConf::null_new(spec.name.clone(), fmt, None);
        let f = build_file_sink(&dummy, &path).await?;
        let stg = HealthController::new();
        let proxy = ASinkTestProxy::new(f, stg);
        Ok(wp_connector_api::SinkHandle::new(Box::new(proxy)))
    }
}

// fast_file 工厂已移除

pub fn register_builtin_factories() {
    crate::connectors::registry::register_sink_factory(BlackHoleFactory);
    crate::connectors::registry::register_sink_factory(FileFactory);
    crate::connectors::registry::register_sink_factory(SyslogFactory);
    crate::connectors::registry::register_sink_factory(TcpFactory);
    crate::connectors::registry::register_sink_factory(TestRescueFactory);
}

#[allow(dead_code)]
pub fn make_blackhole_sink() -> Box<dyn wp_connector_api::AsyncSink> {
    Box::new(BlackHoleSink {})
}

#[cfg(test)]
mod tests {
    use super::*;
    use wp_connector_api::{AsyncRawDataSink, AsyncRecordSink, SinkFactory};

    #[tokio::test(flavor = "multi_thread")]
    async fn file_factory_supports_fmt_param() -> anyhow::Result<()> {
        let tmp = std::env::temp_dir().join(format!("wp_file_factory_fmt_{}.log", nano_ts()));
        let mut params = toml::value::Table::new();
        params.insert(
            "base".into(),
            toml::Value::String(tmp.parent().unwrap().to_string_lossy().into()),
        );
        params.insert(
            "file".into(),
            toml::Value::String(tmp.file_name().unwrap().to_string_lossy().into()),
        );
        params.insert("fmt".into(), toml::Value::String("json".into()));
        let spec = wp_connector_api::SinkSpec {
            group: String::new(),
            name: "t".into(),
            kind: "file".into(),
            connector_id: String::new(),
            params: wp_connector_api::parammap_from_toml_table(params),
            filter: None,
        };
        let ctx = wp_connector_api::SinkBuildCtx::new(std::env::current_dir().unwrap());
        let init = FileFactory.build(&spec, &ctx).await?;
        // write one record as JSON via record sink
        let mut sink = init.sink;
        let rec = wp_model_core::model::DataRecord::default();
        AsyncRecordSink::sink_record(sink.as_mut(), &rec).await?;
        AsyncRawDataSink::sink_str(sink.as_mut(), "\n").await?; // ensure newline flush
        AsyncRawDataSink::sink_str(sink.as_mut(), "").await?;
        drop(sink);
        let body = std::fs::read_to_string(tmp)?;
        assert!(body.trim_start().starts_with("{"));
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn null_factory_is_noop() -> anyhow::Result<()> {
        let spec = wp_connector_api::SinkSpec {
            group: String::new(),
            name: "n".into(),
            kind: "null".into(),
            connector_id: String::new(),
            params: wp_connector_api::parammap_from_toml_table(toml::value::Table::new()),
            filter: None,
        };
        let ctx = wp_connector_api::SinkBuildCtx::new(std::env::current_dir().unwrap());
        let init = BlackHoleFactory.build(&spec, &ctx).await?;
        let mut sink = init.sink;
        AsyncRawDataSink::sink_str(sink.as_mut(), "hello").await?;
        Ok(())
    }

    fn nano_ts() -> i128 {
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0).into()
    }
}
