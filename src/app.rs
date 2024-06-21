use std::time::{Duration, Instant};

use futures_util::AsyncBufReadExt;
use moka::future::{Cache, CacheBuilder};
use serde::{Deserialize, Serialize};
use surf::{http::Method, Client, RequestBuilder, Response, Url};

use crate::log::{LogItem, StructuredLogItem, TraceId, TraceItem};
use anyhow::{anyhow, bail, Context, Result};

// App state
pub struct App {
    pub traces: Cache<TraceId, TraceItem>,
    pub counter: i64,
    pub should_quit: bool,
    pub client: Client,
    pub req: Response,
}

#[derive(Debug, Deserialize, Serialize)]
struct LogItemJson {
    #[serde(rename = "type")]
    pub log_type: String,
    pub payload: String,
}

impl From<LogItemJson> for LogItem {
    fn from(json: LogItemJson) -> Self {
        Self {
            log_type: json.log_type,
            payload: json.payload.into(),
        }
    }
}

pub trait AppConfig {
    fn base_url(&self) -> Url;
    fn cache_size(&self) -> u64;
    fn time_to_idle(&self) -> Duration;
    fn secret(&self) -> Option<String>;
}

impl App {
    pub async fn new(config: impl AppConfig) -> Result<Self> {
        let base_url = config.base_url();
        let cache_size = config.cache_size();
        let tti = config.time_to_idle();

        let client: Client = surf::Config::new().set_timeout(None).try_into()?;

        let req = RequestBuilder::new(Method::Get, base_url.join("/logs?level=trace")?);
        let req = match config.secret() {
            Some(secret) => req.header("Authorization", format!("Bearer {}", secret)),
            None => req,
        };
        let req = req.build();

        let req = client.send(req).await.map_err(|e| anyhow!(e))?;

        Ok(Self {
            traces: CacheBuilder::new(cache_size).time_to_idle(tti).build(),
            counter: 0,
            should_quit: false,
            client,
            req,
        })
    }

    pub async fn fetch(&mut self) -> Result<()> {
        let mut buf = String::new();

        self.req.read_line(&mut buf).await?;

        if buf.is_empty() {
            bail!("Connection closed by server? Cannot read line.");
        }

        let log_item: LogItemJson =
            serde_json::from_str(&buf).with_context(|| format!("Failed to parse: {}", buf))?;
        let log_item: LogItem = log_item.into();

        println!("{:?}", log_item);

        if let StructuredLogItem::Trace(trace) = log_item.payload {
            let trace_item = TraceItem {
                time: Instant::now(),
                duration: trace.duration,
                content: trace.content,
            };
            self.traces.insert(trace.trace_id, trace_item).await;
        }

        Ok(())
    }
}
