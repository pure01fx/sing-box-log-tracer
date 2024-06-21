use std::time::Instant;

use anyhow::{anyhow, ensure, Result};
use serde::{Deserialize, Serialize};

pub type TraceId = u32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogItem {
    pub log_type: String,
    pub payload: StructuredLogItem,
}

/// A trivial log item
/// Format: tag: content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrivialLogItem {
    pub tag: String,
    pub content: String,
}

/// A trace log item
/// Format: [trace_id duration] tag: content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceLogItem {
    pub trace_id: TraceId,
    pub duration: String,
    pub content: TrivialLogItem,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StructuredLogItem {
    Trace(TraceLogItem),
    Trivial(TrivialLogItem),
    Raw(String),
}

impl TryFrom<&str> for TrivialLogItem {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.splitn(2, ": ").collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid log item: {}", s));
        }
        Ok(Self {
            tag: parts[0].to_string(),
            content: parts[1].to_string(),
        })
    }
}

impl TryFrom<&str> for TraceLogItem {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self> {
        ensure!(s.starts_with("["), "Invalid log item");
        let s = &s[1..];

        let parts: Vec<&str> = s.splitn(2, "] ").collect();
        ensure!(parts.len() == 2, "Invalid log item");

        let trace_parts: Vec<&str> = parts[0].splitn(2, " ").collect();
        ensure!(trace_parts.len() == 2, "Invalid log item");

        Ok(Self {
            trace_id: trace_parts[0].parse()?,
            duration: trace_parts[1].to_string(),
            content: TrivialLogItem::try_from(parts[1])?,
        })
    }
}

impl From<String> for StructuredLogItem {
    fn from(s: String) -> Self {
        if s.starts_with("[") {
            match TraceLogItem::try_from(&*s) {
                Ok(trace) => Self::Trace(trace),
                Err(e) => {
                    eprintln!("Failed to parse trace log item: {}", e);
                    Self::Raw(s)
                }
            }
        } else {
            match TrivialLogItem::try_from(&*s) {
                Ok(trivial) => Self::Trivial(trivial),
                Err(e) => {
                    eprintln!("Failed to parse trivial log item: {}", e);
                    Self::Raw(s)
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct TraceItem {
    pub time: Instant,
    pub duration: String,
    pub content: TrivialLogItem,
}
