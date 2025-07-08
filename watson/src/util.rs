pub struct LineRangesIter<'a> {
    str: &'a str,
    pos: usize,
}

impl<'a> Iterator for LineRangesIter<'a> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        let rest = &self.str[self.pos..];

        if rest.is_empty() {
            return None;
        }

        let end = match rest.find('\n') {
            Some(end) => self.pos + end,
            None => self.pos + rest.len()
        };

        let range = (self.pos, end);
        self.pos = end + 1;
        Some(range)
    }
}

pub fn line_ranges(str: &str) -> LineRangesIter {
    return LineRangesIter { str, pos: 0 };
}
