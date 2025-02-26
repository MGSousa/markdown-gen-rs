use std::io;
use std::io::{Error, Write};
use Escaping::{InlineCode, Normal};

#[cfg(test)]
mod tests;

/// Specifies string escaping mode
#[derive(Clone, Copy)]
pub enum Escaping {
    /// `` \`*_{}[]()#+-.!`` will be escaped with a backslash
    Normal,
    /// Inline code will be surrounded by enough backticks to escape the contents
    InlineCode,
}

/// Struct for generating Markdown
pub struct Markdown<W: Write> {
    writer: W,
}

impl<W: Write> Markdown<W> {
    /// Creates a new [Markdown](struct.Markdown.html) struct
    ///
    /// # Arguments
    ///
    /// * `writer` - Destination for Markdown data
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Returns the underlying `writer` and consumes the object
    pub fn into_inner(self) -> W {
        self.writer
    }

    /// Writes a [MarkdownWritable](trait.MarkdownWritable.html) to the document
    ///
    /// # Returns
    /// `()` or `std::io::Error` if an error occurred during writing to the underlying writer
    pub fn write<T: MarkdownWritable>(&mut self, element: T) -> Result<(), io::Error> {
        element.write_to(&mut self.writer, false, Normal, None)?;
        Ok(())
    }
}

/// Trait for objects writable to Markdown documents
pub trait MarkdownWritable {
    /// Writes `self` as markdown to `writer`
    ///
    /// # Arguments
    /// * `writer` - Destination writer
    /// * `inner` - `true` if element is inside another element, `false` otherwise
    /// * `escape` - Mode used for escaping string
    /// * `line_prefix` - Prefix written before each line
    ///
    /// # Returns
    /// `()` or `std::io::Error` if an error occurred during writing
    fn write_to(
        &self,
        writer: &mut dyn Write,
        inner: bool,
        escape: Escaping,
        line_prefix: Option<&[u8]>,
    ) -> Result<(), io::Error>;

    /// Counts length of longest streak of `char` in `self`
    ///
    /// # Arguments
    /// * `char` - Character to search for
    /// * `carry` - Length to add to possible occurrence at the beginning
    ///
    /// # Returns
    /// `(count, carry)`
    /// * `count` - Length of longest streak
    /// * `carry` - Length of streak at the end
    fn count_max_streak(&self, char: u8, carry: usize) -> (usize, usize);
}

/// Trait for objects convertible to a Markdown element
pub trait AsMarkdown<'a> {
    /// Converts `self` to [Paragraph](struct.Paragraph.html)
    fn paragraph(self) -> Paragraph<'a>;
    /// Converts `self` to [Heading](struct.Heading.html)
    ///
    /// # Arguments
    /// * `level` - Heading level (1-6)
    fn heading(self, level: usize) -> Heading<'a>;
    /// Converts `self` to [Link](struct.Link.html)
    ///
    /// # Arguments
    /// * `address` - Address which will the link lead to
    fn link_to(self, address: &'a str) -> Link<'a>;

    /// Converts `self` to **bold** [RichText](struct.RichText.html)
    fn bold(self) -> RichText<'a>;

    /// Converts `self` to *italic* [RichText](struct.RichText.html)
    fn italic(self) -> RichText<'a>;

    /// Converts `self` to `code` [RichText](struct.RichText.html)
    fn code(self) -> RichText<'a>;

    /// Converts `self` to [Quote](struct.Quote.html)
    fn quote(self) -> Quote<'a>;
}

//region Paragraph
/// Markdown paragraph
pub struct Paragraph<'a> {
    children: Vec<Box<dyn 'a + MarkdownWritable>>,
}

impl<'a> Paragraph<'a> {
    /// Creates an empty paragraph
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Appends an element to the paragraph
    pub fn append<T: 'a + MarkdownWritable>(mut self, element: T) -> Self {
        self.children.push(Box::new(element));
        self
    }
}

impl MarkdownWritable for &'_ Paragraph<'_> {
    fn write_to(
        &self,
        writer: &mut dyn Write,
        inner: bool,
        escape: Escaping,
        line_prefix: Option<&[u8]>,
    ) -> Result<(), Error> {
        for child in &self.children {
            child.write_to(writer, true, escape, line_prefix)?;
        }
        if !inner {
            write_line_prefixed(writer, b"\n\n", line_prefix)?;
        }
        Ok(())
    }

    fn count_max_streak(&self, char: u8, carry: usize) -> (usize, usize) {
        let mut carry = carry;
        let mut count = 0;
        for child in &self.children {
            let (c, cr) = child.count_max_streak(char, carry);
            count += c;
            carry = cr;
        }
        count += carry;
        (count, 0)
    }
}

impl MarkdownWritable for Paragraph<'_> {
    fn write_to(
        &self,
        writer: &mut dyn Write,
        inner: bool,
        escape: Escaping,
        line_prefix: Option<&[u8]>,
    ) -> Result<(), Error> {
        (&self).write_to(writer, inner, escape, line_prefix)
    }

    fn count_max_streak(&self, char: u8, carry: usize) -> (usize, usize) {
        (&self).count_max_streak(char, carry)
    }
}
//endregion

//region Heading
/// Markdown heading
pub struct Heading<'a> {
    children: Vec<Box<dyn 'a + MarkdownWritable>>,
    level: usize,
}

impl<'a> Heading<'a> {
    /// Creates an empty heading
    ///
    /// # Arguments
    /// * `level` - Heading level (1-6)
    pub fn new(level: usize) -> Self {
        assert!(level > 0 && level <= 6, "Heading level must be range 1-6.");
        Self {
            children: Vec::new(),
            level,
        }
    }

    /// Appends an element to the heading
    pub fn append<T: 'a + MarkdownWritable>(mut self, element: T) -> Self {
        self.children.push(Box::new(element));
        self
    }
}

impl MarkdownWritable for &'_ Heading<'_> {
    fn write_to(
        &self,
        writer: &mut dyn Write,
        inner: bool,
        _escape: Escaping,
        line_prefix: Option<&[u8]>,
    ) -> Result<(), Error> {
        assert!(!inner, "Inner headings are forbidden.");
        let mut prefix = Vec::new();
        prefix.resize(self.level, b'#');
        prefix.push(b' ');
        writer.write_all(&prefix)?;
        for child in &self.children {
            child.write_to(writer, true, Normal, line_prefix)?;
        }
        write_line_prefixed(writer, b"\n", line_prefix)?;
        Ok(())
    }

    fn count_max_streak(&self, char: u8, _carry: usize) -> (usize, usize) {
        let mut carry = 0;
        let mut count = 0;
        for child in &self.children {
            let (c, cr) = child.count_max_streak(char, carry);
            count += c;
            carry = cr;
        }
        (count, carry)
    }
}

impl MarkdownWritable for Heading<'_> {
    fn write_to(
        &self,
        writer: &mut dyn Write,
        inner: bool,
        escape: Escaping,
        line_prefix: Option<&[u8]>,
    ) -> Result<(), Error> {
        (&self).write_to(writer, inner, escape, line_prefix)
    }

    fn count_max_streak(&self, char: u8, carry: usize) -> (usize, usize) {
        (&self).count_max_streak(char, carry)
    }
}
//endregion

//region Table
/// Markdown Table
pub struct Table<'a> {
    gfm: bool,
    columns: Vec<&'a str>,
    rows: Vec<Vec<String>>,
}

impl<'a> Table<'a> {
    /// Creates an empty table
    ///
    /// # Arguments
    /// * `gfm` - check to use GitHub Flavored Markdown Spec (supports HTML)
    /// if not use default spec
    pub fn new(gfm: bool) -> Self {
        Self {
            gfm,
            columns: vec![],
            rows: vec![vec![]],
        }
    }

    /// Add headers to table
    pub fn header(mut self, columns: Vec<&'a str>) -> Self {
        self.columns = columns;
        self
    }

    /// Appends rows to the table
    pub fn rows(mut self, rows: Vec<Vec<String>>) -> Self {
        self.rows = rows;
        self
    }
}

impl MarkdownWritable for &'_ Table<'_> {
    fn write_to(
        &self,
        writer: &mut dyn Write,
        _: bool,
        _: Escaping,
        line_prefix: Option<&[u8]>,
    ) -> Result<(), Error> {
        // Check if is GitHub Flavored Markdown Spec
        match self.gfm {
            true => {
                let mut table = String::from("<table>");
                for (k, column) in self.columns.iter().enumerate() {
                    if k == 0 {
                        table += format!("<thead><tr><th>{}</th>", column).as_str();
                    } else {
                        table += format!("<th>{}</th>", column).as_str();
                    }
                }
                table += "</tr></thead><tbody>";

                for rows in &self.rows {
                    for (r, row) in rows.iter().enumerate() {
                        if r == 0 {
                            table += format!("<tr><td>{}</td>", row).as_str();
                        } else {
                            table += format!("<td>{}</td>", row).as_str();
                            if (rows.len() - 1) == r {
                                table += "</tr>";
                            }
                        }
                    }
                }
                table += "</tbody></table>";
                writer.write_all(table.as_ref())?;
            }
            false => {
                // TODO: add the normal spec here
            }
        }

        write_line_prefixed(writer, b"\n", line_prefix)?;
        Ok(())
    }

    fn count_max_streak(&self, _: u8, _carry: usize) -> (usize, usize) {
        (0, 0)
    }
}

impl MarkdownWritable for Table<'_> {
    fn write_to(
        &self,
        writer: &mut dyn Write,
        inner: bool,
        escape: Escaping,
        line_prefix: Option<&[u8]>,
    ) -> Result<(), Error> {
        (&self).write_to(writer, inner, escape, line_prefix)
    }

    fn count_max_streak(&self, char: u8, carry: usize) -> (usize, usize) {
        (&self).count_max_streak(char, carry)
    }
}
//endregion

//region Link
/// Markdown link
pub struct Link<'a> {
    children: Vec<Box<dyn 'a + MarkdownWritable>>,
    address: &'a str,
}

impl<'a> Link<'a> {
    /// Creates an empty link, which leads to `address`
    pub fn new(address: &'a str) -> Self {
        Self {
            children: Vec::new(),
            address,
        }
    }

    /// Appends an element to the link's text
    pub fn append<T: 'a + MarkdownWritable>(mut self, element: T) -> Self {
        self.children.push(Box::new(element));
        self
    }
}

impl MarkdownWritable for &'_ Link<'_> {
    fn write_to(
        &self,
        writer: &mut dyn Write,
        inner: bool,
        escape: Escaping,
        line_prefix: Option<&[u8]>,
    ) -> Result<(), Error> {
        writer.write_all(b"[")?;
        for child in &self.children {
            child.write_to(writer, true, escape, line_prefix)?;
        }
        writer.write_all(b"](")?;
        self.address.write_to(writer, true, escape, line_prefix)?;
        writer.write_all(b")")?;
        if !inner {
            write_line_prefixed(writer, b"\n", line_prefix)?;
        }
        Ok(())
    }

    fn count_max_streak(&self, char: u8, _carry: usize) -> (usize, usize) {
        let (mut addr, addr_cr) = self.address.count_max_streak(char, 0);
        addr += addr_cr;
        let mut carry = 0;
        let mut count = 0;
        for child in &self.children {
            let (c, cr) = child.count_max_streak(char, carry);
            count += c;
            carry = cr;
        }
        count += carry;
        return if count > addr { (count, 0) } else { (addr, 0) };
    }
}

impl MarkdownWritable for Link<'_> {
    fn write_to(
        &self,
        writer: &mut dyn Write,
        inner: bool,
        escape: Escaping,
        line_prefix: Option<&[u8]>,
    ) -> Result<(), Error> {
        (&self).write_to(writer, inner, escape, line_prefix)
    }

    fn count_max_streak(&self, char: u8, carry: usize) -> (usize, usize) {
        (&self).count_max_streak(char, carry)
    }
}

impl<'a> AsMarkdown<'a> for &'a Link<'a> {
    fn paragraph(self) -> Paragraph<'a> {
        Paragraph::new().append(self)
    }

    fn heading(self, level: usize) -> Heading<'a> {
        Heading::new(level).append(self)
    }

    fn link_to(self, _address: &'a str) -> Link<'a> {
        panic!("Link cannot contain another link.");
    }

    fn bold(self) -> RichText<'a> {
        panic!("Cannot change link's body. Please use 'x.as_bold().as_link_to(...);'");
    }

    fn italic(self) -> RichText<'a> {
        panic!("Cannot change link's body. Please use 'x.as_italic().as_link_to(...);'");
    }

    fn code(self) -> RichText<'a> {
        panic!("Cannot change link's body. Please use 'x.as_code().as_link_to(...);'");
    }

    fn quote(self) -> Quote<'a> {
        Quote::new().append(self)
    }
}

impl<'a> AsMarkdown<'a> for Link<'a> {
    fn paragraph(self) -> Paragraph<'a> {
        Paragraph::new().append(self)
    }

    fn heading(self, level: usize) -> Heading<'a> {
        Heading::new(level).append(self)
    }

    fn link_to(self, _address: &'a str) -> Link<'a> {
        panic!("Link cannot contain another link.");
    }

    fn bold(self) -> RichText<'a> {
        panic!("Cannot change link's body. Please use 'x.as_bold().as_link_to(...);'");
    }

    fn italic(self) -> RichText<'a> {
        panic!("Cannot change link's body. Please use 'x.as_italic().as_link_to(...);'");
    }

    fn code(self) -> RichText<'a> {
        panic!("Cannot change link's body. Please use 'x.as_code().as_link_to(...);'");
    }

    fn quote(self) -> Quote<'a> {
        Quote::new().append(self)
    }
}
//endregion

//region RichText
/// Text styled with **bold**, *italic* or `code`
#[derive(Copy, Clone)]
pub struct RichText<'a> {
    bold: bool,
    italic: bool,
    code: bool,
    text: &'a str,
}

impl<'a> RichText<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            bold: false,
            italic: false,
            code: false,
            text,
        }
    }
}

impl MarkdownWritable for &'_ RichText<'_> {
    fn write_to(
        &self,
        writer: &mut dyn Write,
        inner: bool,
        mut escape: Escaping,
        line_prefix: Option<&[u8]>,
    ) -> Result<(), Error> {
        let mut symbol = Vec::new();
        if self.bold {
            symbol.extend_from_slice(b"**");
        }
        if self.italic {
            symbol.push(b'*');
        }
        if self.code {
            let (mut ticks_needed, carry) = self.text.count_max_streak(b'`', 0);
            ticks_needed += 1 + carry;
            symbol.extend(vec![b'`'; ticks_needed]);
            symbol.push(b' ');
            escape = InlineCode;
        }

        writer.write_all(&symbol)?;
        self.text.write_to(writer, true, escape, line_prefix)?;
        symbol.reverse();
        writer.write_all(&symbol)?;

        if !inner {
            write_line_prefixed(writer, b"\n\n", line_prefix)?;
        }
        Ok(())
    }

    fn count_max_streak(&self, char: u8, _carry: usize) -> (usize, usize) {
        let (res, cr) = self.text.count_max_streak(char, 0);
        (res + cr, 0)
    }
}

impl MarkdownWritable for RichText<'_> {
    fn write_to(
        &self,
        writer: &mut dyn Write,
        inner: bool,
        escape: Escaping,
        line_prefix: Option<&[u8]>,
    ) -> Result<(), Error> {
        (&self).write_to(writer, inner, escape, line_prefix)
    }

    fn count_max_streak(&self, char: u8, carry: usize) -> (usize, usize) {
        (&self).count_max_streak(char, carry)
    }
}

impl<'a> AsMarkdown<'a> for &'a RichText<'a> {
    fn paragraph(self) -> Paragraph<'a> {
        Paragraph::new().append(self)
    }

    fn heading(self, level: usize) -> Heading<'a> {
        Heading::new(level).append(self)
    }

    fn link_to(self, address: &'a str) -> Link<'a> {
        Link::new(address).append(self)
    }

    fn bold(self) -> RichText<'a> {
        let mut clone = *self;
        clone.bold = true;
        *self
    }

    fn italic(self) -> RichText<'a> {
        let mut clone = *self;
        clone.italic = true;
        *self
    }

    fn code(self) -> RichText<'a> {
        let mut clone = *self;
        clone.code = true;
        *self
    }

    fn quote(self) -> Quote<'a> {
        Quote::new().append(self)
    }
}

impl<'a> AsMarkdown<'a> for RichText<'a> {
    fn paragraph(self) -> Paragraph<'a> {
        Paragraph::new().append(self)
    }

    fn heading(self, level: usize) -> Heading<'a> {
        Heading::new(level).append(self)
    }

    fn link_to(self, address: &'a str) -> Link<'a> {
        Link::new(address).append(self)
    }

    fn bold(mut self) -> RichText<'a> {
        self.bold = true;
        self
    }

    fn italic(mut self) -> RichText<'a> {
        self.italic = true;
        self
    }

    fn code(mut self) -> RichText<'a> {
        self.code = true;
        self
    }

    fn quote(self) -> Quote<'a> {
        Quote::new().append(self)
    }
}
//endregion

//region List
/// Bulleted or numbered list
pub struct List<'a> {
    title: Vec<Box<dyn 'a + MarkdownWritable>>,
    items: Vec<Box<dyn 'a + MarkdownWritable>>,
    numbered: bool,
}

impl<'a> List<'a> {
    /// Creates an empty list
    /// # Arguments
    /// * `numbered` - `true` for numbered list, `false` for bulleted list
    pub fn new(numbered: bool) -> Self {
        Self {
            items: Vec::new(),
            title: Vec::new(),
            numbered,
        }
    }

    /// Append an item to the list title
    pub fn title<T: 'a + MarkdownWritable>(mut self, item: T) -> Self {
        self.title.push(Box::new(item));
        self
    }

    /// Adds an item to the list
    pub fn item<T: 'a + MarkdownWritable>(mut self, item: T) -> Self {
        self.items.push(Box::new(item));
        self
    }
}

impl MarkdownWritable for &'_ List<'_> {
    fn write_to(
        &self,
        writer: &mut dyn Write,
        _inner: bool,
        escape: Escaping,
        line_prefix: Option<&[u8]>,
    ) -> Result<(), Error> {
        for it in &self.title {
            it.write_to(writer, true, escape, line_prefix)?;
        }
        let mut prefix = Vec::new();
        if line_prefix.is_some() {
            prefix.extend_from_slice(line_prefix.unwrap());
        }
        prefix.extend_from_slice(b"   ");

        for it in &self.items {
            if self.numbered {
                write_line_prefixed(writer, b"\n1. ", Some(&prefix))?;
            } else {
                write_line_prefixed(writer, b"\n* ", Some(&prefix))?;
            }

            it.write_to(writer, true, escape, Some(&prefix))?;
        }
        Ok(())
    }

    fn count_max_streak(&self, char: u8, _carry: usize) -> (usize, usize) {
        let mut count = 0;
        for child in &self.items {
            let (c, _) = child.count_max_streak(char, 0);
            if c > count {
                count = c;
            }
        }
        (count, 0)
    }
}

impl<'a> MarkdownWritable for List<'a> {
    fn write_to(
        &self,
        writer: &mut dyn Write,
        inner: bool,
        escape: Escaping,
        line_prefix: Option<&[u8]>,
    ) -> Result<(), Error> {
        (&self).write_to(writer, inner, escape, line_prefix)
    }

    fn count_max_streak(&self, char: u8, carry: usize) -> (usize, usize) {
        (&self).count_max_streak(char, carry)
    }
}

impl<'a> AsMarkdown<'a> for List<'a> {
    fn paragraph(self) -> Paragraph<'a> {
        Paragraph::new().append(self)
    }

    fn heading(self, _level: usize) -> Heading<'a> {
        panic!("Cannot make a Heading from List");
    }

    fn link_to(self, _address: &'a str) -> Link<'a> {
        panic!("Cannot make a Link from List");
    }

    fn bold(self) -> RichText<'a> {
        panic!("Cannot make a List bold");
    }

    fn italic(self) -> RichText<'a> {
        panic!("Cannot make a List italic");
    }

    fn code(self) -> RichText<'a> {
        panic!("Cannot make a List code");
    }

    fn quote(self) -> Quote<'a> {
        Quote::new().append(self)
    }
}
//endregion

//region Quote
/// A quote block
pub struct Quote<'a> {
    children: Vec<Box<dyn 'a + MarkdownWritable>>,
}

impl<'a> Quote<'a> {
    /// Creates an empty quote block
    fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Appends an element to the quote block
    pub fn append<T: 'a + MarkdownWritable>(mut self, element: T) -> Self {
        self.children.push(Box::new(element));
        self
    }
}

impl MarkdownWritable for &'_ Quote<'_> {
    fn write_to(
        &self,
        writer: &mut dyn Write,
        inner: bool,
        escape: Escaping,
        line_prefix: Option<&[u8]>,
    ) -> Result<(), Error> {
        let mut prefix = Vec::new();
        if line_prefix.is_some() {
            prefix.extend_from_slice(line_prefix.unwrap());
        }
        prefix.extend_from_slice(b">");
        if !inner {
            write_line_prefixed(writer, b"\n", line_prefix)?;
        }
        writer.write_all(b">")?;
        for child in &self.children {
            child.write_to(writer, true, escape, Some(&prefix))?;
        }
        if !inner {
            write_line_prefixed(writer, b"\n\n", line_prefix)?;
        }

        Ok(())
    }

    fn count_max_streak(&self, char: u8, _carry: usize) -> (usize, usize) {
        let mut count = 0;
        for child in &self.children {
            let (c, _) = child.count_max_streak(char, 0);
            if c > count {
                count = c;
            }
        }
        (count, 0)
    }
}
impl<'a> MarkdownWritable for Quote<'a> {
    fn write_to(
        &self,
        writer: &mut dyn Write,
        inner: bool,
        escape: Escaping,
        line_prefix: Option<&[u8]>,
    ) -> Result<(), Error> {
        (&self).write_to(writer, inner, escape, line_prefix)
    }

    fn count_max_streak(&self, char: u8, carry: usize) -> (usize, usize) {
        (&self).count_max_streak(char, carry)
    }
}
//endregion

//region String and &str
impl MarkdownWritable for &str {
    fn write_to(
        &self,
        writer: &mut dyn Write,
        inner: bool,
        escape: Escaping,
        line_prefix: Option<&[u8]>,
    ) -> Result<(), Error> {
        match escape {
            Normal => {
                write_escaped(writer, self.as_bytes(), b"\\`*_{}[]()#+-.!", line_prefix)?;
            }
            InlineCode => {
                writer.write_all(self.as_bytes())?;
            }
        }
        if !inner {
            write_line_prefixed(writer, b"\n\n", line_prefix)?;
        }
        Ok(())
    }

    fn count_max_streak(&self, char: u8, carry: usize) -> (usize, usize) {
        let mut iter = self.as_bytes().iter();
        let mut max = 0;
        let mut current = carry;
        loop {
            match iter.next() {
                None => {
                    break;
                }
                Some(ch) => {
                    if *ch == char {
                        current += 1;
                    } else {
                        if current > max {
                            max = current;
                        }
                        current = 0;
                    }
                }
            }
        }
        (max, current)
    }
}

impl<'a> AsMarkdown<'a> for &'a String {
    fn paragraph(self) -> Paragraph<'a> {
        self.as_str().paragraph()
    }

    fn heading(self, level: usize) -> Heading<'a> {
        self.as_str().heading(level)
    }

    fn link_to(self, address: &'a str) -> Link<'a> {
        self.as_str().link_to(address)
    }

    fn bold(self) -> RichText<'a> {
        self.as_str().bold()
    }

    fn italic(self) -> RichText<'a> {
        self.as_str().italic()
    }

    fn code(self) -> RichText<'a> {
        self.as_str().code()
    }

    fn quote(self) -> Quote<'a> {
        self.as_str().quote()
    }
}

impl<'a> AsMarkdown<'a> for &'a str {
    fn paragraph(self) -> Paragraph<'a> {
        Paragraph::new().append(self)
    }

    fn heading(self, level: usize) -> Heading<'a> {
        Heading::new(level).append(self)
    }

    fn link_to(self, address: &'a str) -> Link<'a> {
        Link::new(address).append(self)
    }

    fn bold(self) -> RichText<'a> {
        RichText::new(self).bold()
    }

    fn italic(self) -> RichText<'a> {
        RichText::new(self).italic()
    }

    fn code(self) -> RichText<'a> {
        RichText::new(self).code()
    }

    fn quote(self) -> Quote<'a> {
        Quote::new().append(self)
    }
}
//endregion

fn write_escaped<W: Write + ?Sized>(
    writer: &mut W,
    mut data: &[u8],
    escape: &[u8],
    line_prefix: Option<&[u8]>,
) -> Result<(), Error> {
    loop {
        let slice_at = data.iter().position(|x| escape.contains(x));
        match slice_at {
            Option::None => {
                write_line_prefixed(writer, &data, line_prefix)?;
                return Ok(());
            }
            Some(slice_at) => {
                write_line_prefixed(writer, &data[..slice_at], line_prefix)?;
                writer.write_all(b"\\")?;
                write_line_prefixed(writer, &data[slice_at..slice_at + 1], line_prefix)?;
                data = &data[slice_at + 1..];
            }
        }
    }
}

fn write_line_prefixed<W: Write + ?Sized>(
    writer: &mut W,
    mut data: &[u8],
    line_prefix: Option<&[u8]>,
) -> Result<(), Error> {
    match line_prefix {
        None => {
            writer.write_all(data)?;
        }
        Some(line_prefix) => loop {
            let slice_at = data.iter().position(|x| *x == b'\n');
            match slice_at {
                Option::None => {
                    writer.write_all(&data)?;
                    break;
                }
                Some(slice_at) => {
                    writer.write_all(&data[..slice_at + 1])?;
                    writer.write_all(line_prefix)?;
                    data = &data[slice_at + 1..];
                }
            }
        },
    }

    Ok(())
}
