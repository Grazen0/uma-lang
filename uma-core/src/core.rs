use std::{
    fs, io,
    ops::{Index, Range},
    path::Path,
    slice::SliceIndex,
};

use crate::util::{Position, Span};

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

    pub fn end_pos(&self) -> Position {
        self.byte_to_pos(self.contents.len())
    }

    pub fn contents(&self) -> &str {
        &self.contents
    }

    pub fn count_chars(&self, range: impl SliceIndex<str, Output = str>) -> usize {
        self.contents[range].chars().count()
    }

    // pub fn span_contents(&self, span: &Span) -> &str {}

    pub fn byte_to_pos(&self, byte_pos: usize) -> Position {
        let line = self
            .line_starts
            .binary_search(&byte_pos)
            .unwrap_or_else(|i| i - 1);

        let col = byte_pos - self.line_starts[line];
        Position::new(line, col)
    }

    pub fn pos_to_byte(&self, pos: &Position) -> usize {
        let line_start = self.line_starts[pos.line];
        line_start + pos.col
    }

    pub fn span_to_bytes(&self, span: &Span) -> Range<usize> {
        let start = self.pos_to_byte(&span.start);
        let end = self.pos_to_byte(&span.end);
        start..end
    }

    pub fn line_bytes(&self, line: usize) -> Range<usize> {
        let start = self.line_starts[line];
        let end = self
            .line_starts
            .get(line + 1)
            .copied()
            .unwrap_or(self.contents.len());

        start..end
    }
}

impl Index<Span> for SourceFile {
    type Output = str;

    fn index(&self, index: Span) -> &Self::Output {
        &self.contents[self.span_to_bytes(&index)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_file_line() {
        let src = SourceFile::from_contents(String::from("hello\nworld\n!\n\nfoo"));
        assert_eq!(vec![0, 6, 12, 14, 15], src.line_starts);

        assert_eq!(Position::new(0, 0), src.byte_to_pos(0));
        assert_eq!(Position::new(0, 2), src.byte_to_pos(2));
        assert_eq!(Position::new(0, 4), src.byte_to_pos(4));
        assert_eq!(Position::new(0, 5), src.byte_to_pos(5));

        assert_eq!(Position::new(1, 0), src.byte_to_pos(6));
        assert_eq!(Position::new(1, 2), src.byte_to_pos(8));
        assert_eq!(Position::new(1, 5), src.byte_to_pos(11));

        assert_eq!(Position::new(2, 0), src.byte_to_pos(12));

        assert_eq!(Position::new(3, 0), src.byte_to_pos(14));

        assert_eq!(Position::new(4, 0), src.byte_to_pos(15));
        assert_eq!(Position::new(4, 3), src.byte_to_pos(src.contents.len()));
    }
}
