//! Rescue 数据管理模块：提供 rescue 目录的统计、检查等功能。

mod stat;

pub use stat::{
    scan_rescue_stat, RescueFileStat, RescueStatSummary, SinkRescueStat,
};
