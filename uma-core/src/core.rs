use std::{fs, io, ops::Range, path::Path, slice::SliceIndex};

#[derive(Debug, Clone)]
pub struct SourceFile {
    contents: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    pub fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let contents = fs::read_to_string(path.as_ref())?;
        Ok(Self::from_contents(contents))
    }

    pub fn from_contents(contents: String) -> Self {
        let mut line_starts = vec![0];

        for (i, ch) in contents.char_indices() {
            if ch == '\n' {
                line_starts.push(i + ch.len_utf8());
            }
        }

        Self {
            contents,
            line_starts,
        }
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn end_pos(&self) -> (usize, usize) {
        self.byte_to_line(self.contents.len())
    }

    pub fn contents(&self) -> &str {
        &self.contents
    }

    pub fn count_chars(&self, range: impl SliceIndex<str, Output = str>) -> usize {
        self.contents[range].chars().count()
    }

    pub fn byte_to_line(&self, byte_pos: usize) -> (usize, usize) {
        let line = self
            .line_starts
            .binary_search(&byte_pos)
            .unwrap_or_else(|i| i - 1);

        let col = byte_pos - self.line_starts[line];
        (line, col)
    }

    pub fn line_to_byte_range(&self, line: usize) -> Range<usize> {
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
        let src = SourceFile::from_contents(String::from("hello\nworld\n!\n\nfoo"));
        assert_eq!(vec![0, 6, 12, 14, 15], src.line_starts);

        assert_eq!((0, 0), src.byte_to_line(0));
        assert_eq!((0, 2), src.byte_to_line(2));
        assert_eq!((0, 4), src.byte_to_line(4));
        assert_eq!((0, 5), src.byte_to_line(5));

        assert_eq!((1, 0), src.byte_to_line(6));
        assert_eq!((1, 2), src.byte_to_line(8));
        assert_eq!((1, 5), src.byte_to_line(11));

        assert_eq!((2, 0), src.byte_to_line(12));

        assert_eq!((3, 0), src.byte_to_line(14));

        assert_eq!((4, 0), src.byte_to_line(15));
        assert_eq!((4, 3), src.byte_to_line(src.contents.len()));
    }
}
