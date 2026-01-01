use crate::structure::Protocol;
use educe::Educe;
use orion_conf::error::{ConfIOReason, OrionConfResult};
use orion_error::{ToStructError, UvsValidationFrom};

#[derive(Educe, Serialize, Deserialize, PartialEq, Clone)]
#[educe(Debug, Default)]
pub struct SyslogSinkConf {
    #[educe(Default = "127.0.0.1")]
    pub(crate) addr: String,
    #[educe(Default = 514)]
    pub(crate) port: usize,
    pub protocol: Protocol,
    #[serde(default)]
    pub app_name: Option<String>,
}

impl SyslogSinkConf {
    pub fn addr_str(&self) -> String {
        format!("{}:{}", self.addr, self.port)
    }

    pub fn resolved_app_name(&self, fallback: &str) -> String {
        self.app_name
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| fallback.to_string())
    }
}

// ---------------- Syslog Source Config ----------------

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub struct SyslogSourceConf {
    pub key: String,
    pub addr: String,
    pub port: u16,
    pub protocol: Protocol,
    #[serde(default = "SyslogSourceConf::tcp_read_bytes_default")]
    pub tcp_recv_bytes: usize,
    pub enable: bool,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl Default for SyslogSourceConf {
    fn default() -> Self {
        Self {
            key: "syslog_1".to_string(),
            addr: "0.0.0.0".to_string(),
            port: 514,
            protocol: Protocol::UDP,
            tcp_recv_bytes: 10_485_760, // 10 MiB default read bytes per cycle
            enable: false,
            tags: Vec::new(),
        }
    }
}

impl SyslogSourceConf {
    pub fn addr_str(&self) -> String {
        format!("{}:{}", self.addr, self.port)
    }
    fn tcp_read_bytes_default() -> usize {
        10_485_760
    }
}

impl crate::structure::Validate for SyslogSourceConf {
    fn validate(&self) -> OrionConfResult<()> {
        if self.addr.trim().is_empty() {
            return ConfIOReason::from_validation("syslog.addr must not be empty").err_result();
        }
        if self.port == 0 {
            return ConfIOReason::from_validation("syslog.port must be in 1..=65535").err_result();
        }
        if matches!(self.protocol, Protocol::TCP) && self.tcp_recv_bytes == 0 {
            return ConfIOReason::from_validation("syslog.tcp_recv_bytes must be > 0 for TCP")
                .err_result();
        }
        Ok(())
    }
}
