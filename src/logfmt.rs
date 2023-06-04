use std::collections::HashMap;
use crate::Log;
use anyhow::Result;

pub fn parse_logfmt(mut log: String) -> Result<Log> {
    let mut level = None;
    let mut logger_name = None;
    let mut data = HashMap::new();
    let mut log_message_start = 0usize; // markers for simplified message, that ignores the metadata
    let mut log_message_end = None;
    let line = log.as_str();
    let mut s = line.char_indices().peekable();
    let mut cur_token_idx = 0;
    while let Some(&(i, c)) = s.peek() {
        if c == '=' {
            break;
        }
        let (i, c) = s.next().unwrap();
        let at_colon = c == ':' && matches!(s.peek(), Some((_, ' ')));
        if c == ' ' || at_colon {
            match &line[cur_token_idx..i] {
                "INFO" | "WARN" | "WARNING" | "ERROR" | "DEBUG" | "TRACE" | "LOG" => {
                    if level.is_none() {
                        level = Some(line[cur_token_idx..i].to_string());
                    }
                    if cur_token_idx == log_message_start {
                        log_message_start = i + 1;
                    }
                }
                s if s.contains(&['.', ':']) => {
                    // if cur_token_idx != log_message_start, then we're already "in the message"
                    // and we should continue rather than assuming this is the logger name
                    if cur_token_idx == log_message_start {
                        log_message_start = i + 1;
                        if logger_name.is_none() {
                            logger_name = Some(line[cur_token_idx..i].to_string());
                        }
                    }
                }
                _ => {}
            }
            cur_token_idx = i + 1;
        }
        if at_colon {
            if log_message_start > 0 {
                log_message_start += 1;
            }
            break;
        }
    }
    // assume we're in message and/or key value section now.
    while let Some((i, c)) = s.next() {
        match c {
            ' ' => {
                log_message_end = None;
                cur_token_idx = i + 1;
            }
            '=' => {
                if log_message_end.is_none() && cur_token_idx > 0 {
                    log_message_end = Some(cur_token_idx - 1);
                }
                let key = &line[cur_token_idx..i];
                cur_token_idx = i + 1;
                let mut end_token_idx = line.len();
                'outer: while let Some((i, ch)) = s.next() {
                    match ch {
                        ' ' => {
                            end_token_idx = i;
                            break 'outer;
                        }
                        '"' => {
                            cur_token_idx = i + 1;
                            while let Some((i, ch)) = s.next() {
                                match ch {
                                    '\\' => {
                                        s.next();
                                        continue;
                                    }
                                    '"' => {
                                        end_token_idx = i;
                                        break 'outer;
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
                let value = &line[cur_token_idx..end_token_idx];
                data.insert(key.to_string(), value.into());
                cur_token_idx = end_token_idx + 1;
            }
            _ => {}
        }
    }
    println!("{} {:?}", log_message_start, log_message_end);
    if let Some(end) = log_message_end {
        log.truncate(end);
    }
    log = log.split_off(log_message_start);
    Ok(Log {
        dt: None,
        level,
        name: logger_name,
        message: log,
        platform: None,
        extension: None,
        data,
    })
}