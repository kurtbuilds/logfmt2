use std::str::Chars;
use std::time::Duration;

/// Error parsing human-friendly duration
#[derive(Debug, PartialEq, Clone)]
pub enum Error {
    /// Invalid character during parsing
    ///
    /// More specifically anything that is not alphanumeric is prohibited
    ///
    /// The field is an byte offset of the character in the string.
    InvalidCharacter(usize),
    /// Non-numeric value where number is expected
    ///
    /// This usually means that either time unit is broken into words,
    /// e.g. `m sec` instead of `msec`, or just number is omitted,
    /// for example `2 hours min` instead of `2 hours 1 min`
    ///
    /// The field is an byte offset of the errorneous character
    /// in the string.
    NumberExpected(usize),
    /// Unit in the number is not one of allowed units
    ///
    /// See documentation of `parse_duration` for the list of supported
    /// time units.
    ///
    /// The two fields are start and end (exclusive) of the slice from
    /// the original string, containing errorneous value
    UnknownUnit {
        /// The unit verbatim
        unit: String,
        /// A number associated with the unit
        value: u64,
    },
    /// The numeric value is too large
    ///
    /// Usually this means value is too large to be useful. If user writes
    /// data in subsecond units, then the maximum is about 3k years. When
    /// using seconds, or larger units, the limit is even larger.
    NumberOverflow,
    /// The value was an empty string (or consists only whitespace)
    Empty,
}

trait OverflowOp: Sized {
    fn mul(self, other: Self) -> Result<Self, Error>;
    fn add(self, other: Self) -> Result<Self, Error>;
}

impl OverflowOp for u64 {
    fn mul(self, other: Self) -> Result<Self, Error> {
        // Ok(self * other)
        self.checked_mul(other).ok_or(Error::NumberOverflow)
    }
    fn add(self, other: Self) -> Result<Self, Error> {
        // Ok(self + other)
        self.checked_add(other).ok_or(Error::NumberOverflow)
    }
}

struct Parser<'a> {
    src: &'a str,
    current: (u64, u64),
}

impl<'a> Parser<'a> {
    fn add_unit(&mut self, n: u64, d: u64, unit: &str) -> Result<(), Error> {
        let unit = match unit {
            "nanos" | "nsec" | "ns" => 1e-9,
            "usec" | "us" | "µs" => 1e-6,
            "millis" | "msec" | "ms" => 1e-3,
            "seconds" | "second" | "secs" | "sec" | "s" => 1.,
            "minutes" | "minute" | "min" | "mins" | "m" => 60.,
            "hours" | "hour" | "hr" | "hrs" | "h" => 3600.,
            "days" | "day" | "d" => 86400.,
            "weeks" | "week" | "w" => 86400. * 7.,
            "months" | "month" | "M" => 2_630_016., // 30.44d
            "years" | "year" | "y" => 31_557_600., // 365.25d
            _ => {
                return Err(Error::UnknownUnit {
                    unit: unit.to_string(),
                    value: n,
                });
            }
        };
        let places = d.to_string().len();
        let d = d as f64 / 10f64.powi(places as i32);
        let n = n as f64 + d;
        let sec = unit * n;
        let rounded = sec.fract();
        let ns = (sec.fract() * 1e9).round() as u64;
        self.current.0 += sec as u64;
        self.current.1 += ns;
        Ok(())
    }

    fn parse(mut self) -> Result<Duration, Error> {
        let mut ch = self.src.char_indices().peekable();
        loop {
            let mut n = 0u64;
            let mut d = 0u64;
            'outer: while let Some((i, c)) = ch.next() {
                match c {
                    '0'..='9' => {
                        n = n.checked_mul(10)
                            .and_then(|x| x.checked_add(c as u64 - '0' as u64))
                            .ok_or(Error::NumberOverflow)?;
                    }
                    '.' => {
                        while let Some(&(i, c)) = ch.peek() {
                            match c {
                                '0'..='9' => {
                                    d = d.checked_mul(10)
                                        .and_then(|x| x.checked_add(c as u64 - '0' as u64))
                                        .ok_or(Error::NumberOverflow)?;
                                }
                                _ => break 'outer,
                            }
                            ch.next();
                        }
                    }
                    c if c.is_whitespace() => continue,
                    _ => return Err(Error::InvalidCharacter(i)),
                }
            }
            let start = ch.peek().ok_or(Error::Empty)?.0;
            while let Some((i, c)) = ch.next() {
                match c {
                    c if c.is_ascii_alphabetic() => {
                        continue;
                    },
                    _ => return Err(Error::InvalidCharacter(i)),
                }
            }
            let end = ch.peek().map(|&(i, _)| i).unwrap_or(self.src.len());
            self.add_unit(n, d, &self.src[start..end])?;
            if ch.peek().is_none() {
                break;
            }
        }
        Ok(Duration::new(self.current.0, self.current.1 as u32))
    }
}

pub fn parse_duration(s: &str) -> Result<Duration, Error> {
    Parser {
        src: s,
        current: (0, 0),
    }.parse()
}

#[cfg(test)]
mod test {
    use std::ops::Add;
    use super::*;

    #[test]
    fn test_decimal() {
        let s = "189.457178ms";
        let s = parse_duration(s).unwrap();
        assert_eq!(s, Duration::from_millis(189).add(Duration::from_nanos(457178)));
    }

    #[test]
    fn test_rounding() {
        let s = "100.32ms";
        let s = parse_duration(s).unwrap();
        assert_eq!(s, Duration::from_millis(100).add(Duration::from_micros(320)));
    }

    #[test]
    fn test_error() {
        assert!(parse_duration("123.1234").is_err());
        assert!(parse_duration("127.0.0.1").is_err());
    }
}