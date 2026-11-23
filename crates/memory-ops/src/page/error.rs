#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("line {line}: {kind}")]
pub struct PageError {
    line: usize,
    kind: PageErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PageErrorKind {
    #[error("frontmatter is not closed")]
    UnclosedFrontmatter,
    #[error("frontmatter is not valid yaml")]
    Frontmatter { reason: String },
    #[error("field value must be a scalar or a list of scalars")]
    FieldNotScalar { key: String },
    #[error("field name must be 1 to 64 characters")]
    FieldKeyLength { key: String },
    #[error("field value is longer than 2000 bytes")]
    FieldValueTooLong { key: String },
    #[error("timeline line must start with a valid YYYY-MM-DD date")]
    TimelineDate,
    #[error("timeline date must be followed by ` | `")]
    TimelineSeparator,
    #[error("timeline line needs a source before ` — `")]
    TimelineSource,
    #[error("timeline line needs text after ` — `")]
    TimelineText,
    #[error("timeline line is blank")]
    TimelineBlankLine,
}

impl PageError {
    pub(crate) fn new(line: usize, kind: PageErrorKind) -> Self {
        Self { line, kind }
    }

    pub fn line(&self) -> usize {
        self.line
    }

    pub fn kind(&self) -> &PageErrorKind {
        &self.kind
    }
}
