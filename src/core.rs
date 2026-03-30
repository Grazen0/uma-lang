use std::{fs, io, ops::Range, path::Path, slice::SliceIndex};

#[derive(Debug, Clone)]
pub struct SourceText {
    contents: String,
    line_starts: Vec<usize>,
}

impl SourceText {
    pub fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let contents = fs::read_to_string(path.as_ref())?;
        Ok(Self::from_contents(contents))
    }

    fn from_contents(contents: String) -> Self {
        let mut line_positions = vec![0];

        for (i, ch) in contents.char_indices() {
            if ch == '\n' {
                line_positions.push(i + 1);
            }
        }

        Self {
            contents,
            line_starts: line_positions,
        }
    }

    pub fn contents(&self) -> &str {
        &self.contents
    }

    pub fn count_chars(&self, range: impl SliceIndex<str, Output = str>) -> usize {
        self.contents[range].chars().count()
    }

    pub fn find_line(&self, byte_pos: usize) -> usize {
        self.line_starts
            .binary_search(&byte_pos)
            .unwrap_or_else(|i| i - 1)
    }

    pub fn line_range_bytes(&self, line: usize) -> Range<usize> {
        let start = self.line_starts[line];
        let end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.contents.len());

        start..end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_file_line() {
        let src = SourceText::from_contents(String::from("hello\nworld\n!\n\nfoo"));
        assert_eq!(vec![0, 6, 12, 14, 15], src.line_starts);

        assert_eq!(0, src.find_line(0));
        assert_eq!(0, src.find_line(2));
        assert_eq!(0, src.find_line(4));
        assert_eq!(0, src.find_line(5));

        assert_eq!(1, src.find_line(6));
        assert_eq!(1, src.find_line(8));
        assert_eq!(1, src.find_line(11));

        assert_eq!(2, src.find_line(12));

        assert_eq!(3, src.find_line(14));

        assert_eq!(4, src.find_line(15));
        assert_eq!(4, src.find_line(src.contents.len()));
    }
}
