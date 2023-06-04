#![allow(unused)]
// https://github.com/rustls/rustls/blob/main/examples/src/bin/tlsserver-mio.rs

mod fast;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::Result;

pub enum ParseStrategy {
    Json,
    Logfmt,
}

pub enum OuterStrategy {
    Direct(ParseStrategy),
    JsonWrapping(ParseStrategy),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Log {
    dt: String,
    level: Option<String>,
    #[serde(default)]
    name: Option<String>,
    message: String,
    platform: String,
    #[serde(flatten)]
    extension: Value
}


fn heuristic_reparse(log: &mut Log) {
    let mut new_start = 0usize;
    let mut new_end = None;
    let line = log.message.as_str();
    let mut s = line.char_indices().peekable();
    let mut last_idx = 0;
    while let Some((i, ch)) = s.next() {
        match ch {
            // if char is a space, then we tokenize. check if last word was a level, a logger name, or something else
            ' ' => {
                match &line[last_idx..i] {
                    "INFO" | "WARN" | "WARNING" | "ERROR" | "DEBUG" | "TRACE" => {
                        if log.level.is_none() {
                            log.level = Some(line[last_idx..i].to_string());
                        }
                        if last_idx == new_start {
                            new_start = i + 1;
                        }
                        last_idx = i + 1;
                    }
                    s if s.contains(&['.', ':']) => {
                        if log.name.is_none() {
                            log.name = Some(line[last_idx..i].to_string());
                        }
                        if last_idx == new_start {
                            new_start = i + 1;
                        }
                        last_idx = i + 1;
                    }
                    _ => {
                        last_idx = i + 1;
                    }
                }
            }
            // if char is =, then assume its a logfmt key=value pair
            '=' => {
                if new_end.is_none() && last_idx > 0 {
                    new_end = Some(last_idx - 1);
                }
                let key = &line[last_idx..i];
                last_idx = i + 1;
                'outer: while let Some((i, ch)) = s.next() {
                    match ch {
                        ' ' => {
                            let value = &line[last_idx..i];
                            last_idx = i + 1;
                            log.extension[key] = serde_json::Value::String(value.to_string());
                            break
                        }
                        '"' => {
                            last_idx = i + 1;
                            while let Some((i, ch)) = s.next() {
                                match ch {
                                    '\\' => {
                                        s.next();
                                        continue;
                                    }
                                    '"' => {
                                        let value = &line[last_idx..i];
                                        last_idx = i + 1;
                                        log.extension[key] = serde_json::Value::String(value.to_string());
                                        break 'outer;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(new_end) = new_end {
        log.message.truncate(new_end);
    }
    log.message = log.message.split_off(new_start);
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_logparse() {
        let a = r#"
        {"dt":"2023-06-04T01:42:46.344493Z","level":"info","message":"INFO server::onboarding::location_availability: Updated profile with postal code tz=America/Chicago area=- postal_code=76133 req=00djxys6h3gzskbwhwy5zk_pgkx user=1023","platform":"Syslog","syslog":{"appname":"web-2q9fl","facility":"kern","host":"jyve-next","hostname":"jyve-next","msgid":"web-2q9fl","procid":1,"source_ip":"10.0.9.247","version":1}}
        "#.trim();
        let mut s = serde_json::from_str::<Log>(a).unwrap();
        heuristic_reparse(&mut s);
        println!("{:#?}", s);
        assert_eq!(1, 0);
    }
}