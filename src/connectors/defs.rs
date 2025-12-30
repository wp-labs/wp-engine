use wp_conf::connectors::ConnectorDef;
use wp_connector_api::SourceDefProvider;

pub fn builtin_sink_defs() -> Vec<ConnectorDef> {
    crate::sinks::builtin_factories::builtin_sink_defs()
}

pub fn builtin_source_defs() -> Vec<ConnectorDef> {
    vec![
        crate::sources::file::FileSourceFactory.source_def(),
        crate::sources::syslog::SyslogSourceFactory::new().source_def(),
        crate::sources::tcp::TcpSourceFactory.source_def(),
    ]
}
