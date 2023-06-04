use std::num::NonZeroUsize;

struct Range {
    start: usize,
    end: NonZeroUsize,
}

impl Range {
    pub fn end(&self) -> usize {
        self.end.into()
    }
}

pub struct Log {
    line: String,
    pairs: Vec<(Range, Range)>,
    level: Option<Range>,
    path: Option<Range>,
    message: Option<Range>,
}

impl Log {
    pub fn new(line: String) -> Log {
        Log {
            line,
            pairs: Vec::new(),
            level: None,
            path: None,
            message: None,
        }
    }

    pub fn path(&self) -> Option<&str> {
        let path = self.path.as_ref()?;
        Some(&self.line[path.start..path.end()])
    }

    pub fn level(&self) -> Option<&str> {
        let level = self.level.as_ref()?;
        Some(&self.line[level.start..level.end()])
    }

    pub fn message(&self) -> Option<&str> {
        let message = self.message.as_ref()?;
        Some(&self.line[message.start..message.end()])
    }

    pub fn pairs(&self) -> Vec<(&str, &str)> {
        self.pairs.iter().map(|(key, value)| {
            let key = &self.line[key.start..key.end()];
            let value = &self.line[value.start..value.end()];
            (key, value)
        }).collect()
    }
}


#[cfg(test)]
mod test {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_size() {
        assert_eq!(size_of::<Range>(), size_of::<Option<Range>>());
    }
}
