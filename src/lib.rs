#![allow(unused)]
mod fast;
mod humantime;
mod json;
mod logfmt;

use std::collections::HashMap;
use std::fmt::Formatter;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::Result;
use indexmap::IndexMap;
use crate::logfmt::parse_logfmt;

pub struct Parser {
     strategy: Strategy,
}

#[derive(Serialize, Deserialize)]
pub enum DataValue {
    String(String),
    F64(f64),
    I64(i64),
    Duration(std::time::Duration),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Log {
    dt: Option<String>,
    level: Option<String>,
    #[serde(default)]
    name: Option<String>,
    pub message: String,
    platform: Option<String>,
    #[serde(default, flatten)]
    extension: Option<Value>,
    #[serde(default)]
    data: HashMap<String, DataValue>,
}

#[derive(Debug, Copy, Clone)]
pub enum InnerStrategy {
    Json,
    Logfmt,
}

#[derive(Debug, Copy, Clone)]
pub enum Strategy {
    Direct(InnerStrategy),
    Nested {
        outer: InnerStrategy,
        inner: InnerStrategy,
    }
}

impl Parser {
    pub fn nested() -> Self {
        Self {
            strategy: Strategy::Nested {
                outer: InnerStrategy::Json,
                inner: InnerStrategy::Logfmt,
            }
        }
    }

    pub fn new(strategy: Strategy) -> Self {
        Self { strategy }
    }

    pub fn parse(&self, line: String) -> Result<Log> {
        fn parse(line: String, strategy: InnerStrategy) -> Result<Log> {
            use InnerStrategy::*;
            match strategy {
                Json => serde_json::from_str(&line).map_err(|e| e.into()),
                Logfmt => parse_logfmt(line)
            }
        }

        match self.strategy {
            Strategy::Direct(s) => parse(line, s),
            Strategy::Nested { inner, outer } => {
                let mut outer = parse(line, outer)?;
                let line = std::mem::replace(&mut outer.message, "".to_string());
                let inner = parse(line, inner)?;
                outer.data.extend(inner.data);
                outer.name = inner.name;
                if outer.level.is_none() {
                    outer.level = inner.level;
                }
                outer.message = inner.message;
                Ok(outer)
            }
        }
    }
}

impl std::fmt::Debug for DataValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataValue::String(s) => write!(f, "{:?}", s),
            DataValue::F64(float) => write!(f, "{}", float),
            DataValue::I64(int) => write!(f, "{}", int),
            DataValue::Duration(duration) => write!(f, "{:?}", duration),
        }
    }
}

impl std::fmt::Display for DataValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DataValue::String(s) => write!(f, "{}", s),
            DataValue::F64(float) => write!(f, "{}", float),
            DataValue::I64(int) => write!(f, "{}", int),
            DataValue::Duration(duration) => write!(f, "{:?}", duration),
        }
    }
}

impl From<&str> for DataValue {
    fn from(s: &str) -> Self {
        if let Some(int) = s.parse().ok() {
            DataValue::I64(int)
        } else if let Some(float) = s.parse().ok() {
            DataValue::F64(float)
        } else if let Ok(duration) = humantime::parse_duration(&s) {
            DataValue::Duration(duration)
        } else {
            DataValue::String(s.to_string())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_logparse() {
        let json = r#"
        {"dt":"2023-06-04T01:42:46.344493Z","level":"info","message":"INFO server::onboarding::location_availability: Updated profile with postal code tz=America/Chicago area=- postal_code=10001 req=00djxys6h3gzskbwhwy5zk_pgkx user=7","platform":"Syslog","syslog":{"appname":"web-2q9fl","facility":"kern","host":"jyve-next","hostname":"jyve-next","msgid":"web-2q9fl","procid":1,"source_ip":"10.0.9.247","version":1}}
        "#.trim();
        _ = "INFO server::onboarding::location_availability: Updated profile with postal code tz=America/Chicago area=- postal_code=76133 req=00djxys6h3gzskbwhwy5zk_pgkx user=7";
        let log = Parser::nested().parse(json.to_string()).unwrap();
        assert_eq!(log.level, Some("info".to_string()));
        assert_eq!(log.name, Some("server::onboarding::location_availability".to_string()));
        assert_eq!(log.message, "Updated profile with postal code".to_string());
        assert_eq!(log.data["tz"].to_string(), "America/Chicago");
        assert_eq!(log.data["area"].to_string(), "-");
        assert_eq!(log.data["postal_code"].to_string(), "10001");
        assert_eq!(log.data["req"].to_string(), "00djxys6h3gzskbwhwy5zk_pgkx");
        assert_eq!(log.data["user"].to_string(), "7");
    }

    #[test]
    fn test_render_postgres_log() {
        let json = r#"
        {
    "dt": "2023-06-04T01:41:12.519614Z",
    "level": "info",
    "message": "[4-1] user=db_user,db=foo,app=[unknown],client=2.2.2.2,LOG:  disconnection: session time: 0:00:00.153 user=db_user database=foo host=1.1.1.1 port=1",
    "platform": "Syslog",
    "syslog": {
        "appname": "dpg-4hdwr",
        "facility": "kern",
        "host": "dpg-ccuecsl3t398coemnq80-a-64647bb8b9-4hdwr",
        "hostname": "dpg-ccuecsl3t398coemnq80-a-64647bb8b9-4hdwr",
        "msgid": "dpg-4hdwr",
        "procid": 1,
        "source_ip": "10.0.9.247",
        "version": 1
    }
}"#;
        let log = Parser::nested().parse(json.to_string()).unwrap();
        assert_eq!(log.data["user"].to_string(), "db_user");
        assert_eq!(log.data["database"].to_string(), "foo");
        assert_eq!(log.data["port"].to_string(), "1");
        assert_eq!(log.data["host"].to_string(), "1.1.1.1");
        assert!(log.data.get("client").is_none());
        assert_eq!(log.message, "[4-1] user=db_user,db=foo,app=[unknown],client=2.2.2.2,LOG:  disconnection: session time: 0:00:00.153");
    }

    #[test]
    fn test_render_request_completed_log() {
        let json = r#"
        {
    "dt": "2023-06-04T01:41:12.519614Z",
    "level": "info",
    "message": "Request completed latency=100.32ms",
    "platform": "Syslog",
    "syslog": {
        "appname": "dpg-4hdwr",
        "facility": "kern",
        "host": "dpg-ccuecsl3t398coemnq80-a-64647bb8b9-4hdwr",
        "hostname": "dpg-ccuecsl3t398coemnq80-a-64647bb8b9-4hdwr",
        "msgid": "dpg-4hdwr",
        "procid": 1,
        "source_ip": "10.0.9.247",
        "version": 1
    }
}"#;
        let log = Parser::nested().parse(json.to_string()).unwrap();
        assert_eq!(log.data["latency"].to_string(), "100.32ms");
    }

    // #[test]
    // fn test_tracing_spans() {
    //     let s = "2022-09-14T15:47:01.684149Z do_some_work{n=0}: close time.busy=110ms time.idle=5.10µs";
    //     let log = Parser::new(Strategy::Direct(InnerStrategy::Logfmt)).parse(s.to_string()).unwrap();
    //     println!("{:#?}", log);
    //     assert_eq!(1, 0);
    // }
}