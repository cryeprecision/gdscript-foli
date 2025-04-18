pub trait NodeExt {
    fn to_source_span(&self) -> miette::SourceSpan;
    fn to_source_point_start(&self) -> miette::SourceSpan;
    fn to_source_point_end(&self) -> miette::SourceSpan;
    fn text<'a>(&self, source: &'a [u8]) -> &'a str;
}

impl NodeExt for tree_sitter::Node<'_> {
    fn to_source_span(&self) -> miette::SourceSpan {
        miette::SourceSpan::new(
            self.start_byte().into(),
            self.end_byte() - self.start_byte(),
        )
    }

    fn to_source_point_start(&self) -> miette::SourceSpan {
        miette::SourceSpan::new(self.start_byte().into(), 0)
    }

    fn to_source_point_end(&self) -> miette::SourceSpan {
        miette::SourceSpan::new(self.end_byte().into(), 0)
    }

    #[inline]
    fn text<'a>(&self, source: &'a [u8]) -> &'a str {
        self.utf8_text(source).expect("valid utf8")
    }
}
