/// To store the location of the characters in the source code
/// and later to use it to display debug info
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// UTF-8 byte offsets; end is exculsive
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spanned<T> {
    pub node: T,
    pub span: Span
}

/// Information about the line and column
/// Column here counts utf-8 characters
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceLocation {
    /// One-based(starting from 1) line and column
    pub line: usize,
    pub column: usize,
}

/// Stores information about the file from which we have gotten
/// the source
#[derive(Debug, Clone)]
pub struct SourceFile {
    name: String,
    text: String,
    // The offset from which newline starts
    line_starts: Box<[usize]>,
}

impl SourceFile {
    /// Creates a new instance of SourceFile struct which allows you to
    /// later get the information like the location (line and column) of
    /// the character which you can use to report errors to user by 
    /// pin-pointing them to a specific line in a specific file
    #[must_use]
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> Self {
        let text = text.into();
        let mut line_starts = vec![0];

        for(offset, byte) in text.bytes().enumerate() {
            if byte == b'\n'{
                // we got newline in this byte, so next line starts from next offset
                line_starts.push(offset + 1);
            }
        }

        Self {
            name: name.into(),
            text,
            line_starts: line_starts.into_boxed_slice(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// This returns the `SourceLocation` struct instance which has information like
    /// the line of the byte offset from the text you had passed to new() method of 
    /// SourceFile earlier
    /// 
    /// Note that the '\n' also has its byte offset
    /// ## Example
    /// "
    /// line1\n
    /// line2
    /// "
    /// 
    /// Here, byte offset of '1' is 4 and '\n' is 5, so this would return
    /// (SourceFile { line: 1, column: 5 }) for '1'  
    /// And (SourceFile { line: 1, column: 6 }) for '\n'  
    pub fn location(&self, byte_offset: usize) -> Option<SourceLocation> {
        // If the offset is out of the length of the text we have; we can't return
        // its location
        if byte_offset > self.text.len() || !self.text.is_char_boundary(byte_offset) {
            return None;
        }

        // Get the index of the byte offset
        let line_index = self
            .line_starts
            // Here .partition_point() gives us the index of the next element from
            // the list which satisfies the criteria (eg: line_start <= byte_offset)
            .partition_point(|&line_start| line_start <= byte_offset)
            // Since we have index of the element next to the one that fulfills our criteria
            // we do -1 to get the index of the last element that satiffies our criteria
            // 
            // And since we are looking at lines and not columns, the index we get would
            // be the index of the nearest line
            -1;

        let line_start = self.line_starts[line_index];
        let column = self.text[line_start..byte_offset].chars().count() + 1;

        Some(SourceLocation {
            line: line_index + 1,
            column,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_byte_offsets_to_one_based_locations() {
        let source = SourceFile::new("main.rpl", "first_line\nsecond");
        
        assert_eq!(
            source.location(5),
            Some(SourceLocation { line: 1, column: 6 })
        );

        assert_eq!(
            source.location(10),
            Some(SourceLocation { line: 1, column: 11 })
        );

        assert_eq!(
            source.location(16),
            Some(SourceLocation { line: 2, column: 6 })
        );

        assert_eq!(
            source.location(17),
            Some(SourceLocation { line: 2, column: 7 })
        );

        assert_eq!(
            source.location(18),
            None,
        )
    }

    #[test]
    fn counts_columns_by_characters_not_utf8_bytes() {
        // Here, é takes up 2 bytes but is one character,
        // so it should be treated as a single character 
        // despite being of 2 bytes
        let source = SourceFile::new("main.rpl", "a\néx");

        assert_eq!(
            source.location(4),
            Some(SourceLocation { line: 2, column: 2 })
        )
    }
}
