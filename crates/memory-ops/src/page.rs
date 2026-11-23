mod error;
mod fields;
mod timeline;

use time::Date;

pub use error::{PageError, PageErrorKind};
pub(crate) use fields::is_number;
pub use fields::{Field, FieldValue};
pub use timeline::{Source, TimelineEntry};

#[cfg(test)]
mod tests;

const FRONTMATTER_FENCE: &str = "---";
const TIMELINE_HEADING: &str = "## Timeline";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Page {
    head: String,
    fields: Vec<Field>,
    body: String,
    timeline: Option<Timeline>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Timeline {
    heading: String,
    raw: String,
    entries: Vec<TimelineEntry>,
}

impl Page {
    pub fn parse(text: &str) -> Result<Self, PageError> {
        let lines: Vec<&str> = text.split_inclusive('\n').collect();
        let (head, fields, body_start) = parse_head(&lines)?;
        let heading_index = lines
            .iter()
            .enumerate()
            .skip(body_start)
            .rev()
            .find(|(_, line)| line.trim_end() == TIMELINE_HEADING)
            .map(|(index, _)| index);
        let Some(heading_index) = heading_index else {
            return Ok(Self {
                head,
                fields,
                body: lines.get(body_start..).unwrap_or_default().concat(),
                timeline: None,
            });
        };
        let raw = lines.get(heading_index + 1..).unwrap_or_default().concat();
        let entries = parse_entries(&raw, heading_index + 2)?;
        Ok(Self {
            head,
            fields,
            body: lines
                .get(body_start..heading_index)
                .unwrap_or_default()
                .concat(),
            timeline: Some(Timeline {
                heading: lines
                    .get(heading_index)
                    .copied()
                    .unwrap_or_default()
                    .to_owned(),
                raw,
                entries,
            }),
        })
    }

    pub fn to_text(&self) -> String {
        let mut text = format!("{}{}", self.head, self.body);
        if let Some(timeline) = &self.timeline {
            text.push_str(&timeline.heading);
            text.push_str(&timeline.raw);
        }
        text
    }

    pub fn fields(&self) -> &[Field] {
        &self.fields
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn timeline(&self) -> &[TimelineEntry] {
        self.timeline
            .as_ref()
            .map_or(&[], |timeline| timeline.entries.as_slice())
    }

    pub fn append_timeline(
        &mut self,
        date: Date,
        source: &str,
        text: &str,
    ) -> Result<(), PageError> {
        let line_number = self.to_text().lines().count() + 1;
        let entry = TimelineEntry::compose(date, source, text)
            .map_err(|kind| PageError::new(line_number, kind))?;
        let timeline = self.timeline.get_or_insert_with(|| Timeline {
            heading: String::new(),
            raw: String::new(),
            entries: Vec::new(),
        });
        if timeline.heading.is_empty() {
            if !self.body.is_empty() && !self.body.ends_with('\n') {
                self.body.push('\n');
            }
            if !self.body.is_empty() && !self.body.ends_with("\n\n") {
                self.body.push('\n');
            }
            timeline.heading = format!("{TIMELINE_HEADING}\n");
        } else if !timeline.heading.ends_with('\n') {
            timeline.heading.push('\n');
        }
        let kept = timeline.raw.trim_end().len();
        timeline.raw.truncate(kept);
        if !timeline.raw.is_empty() {
            timeline.raw.push('\n');
        }
        timeline.raw.push_str(entry.raw());
        timeline.raw.push('\n');
        timeline.entries.push(entry);
        Ok(())
    }

    pub fn extends_timeline_of(&self, previous: &Page) -> bool {
        let before = previous.timeline();
        let after = self.timeline();
        after.len() >= before.len()
            && before
                .iter()
                .zip(after)
                .all(|(old, new)| old.raw() == new.raw())
    }
}

fn parse_head(lines: &[&str]) -> Result<(String, Vec<Field>, usize), PageError> {
    if lines.first().map(|line| line.trim_end()) != Some(FRONTMATTER_FENCE) {
        return Ok((String::new(), Vec::new(), 0));
    }
    let close = lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| line.trim_end() == FRONTMATTER_FENCE)
        .map(|(index, _)| index)
        .ok_or_else(|| PageError::new(1, PageErrorKind::UnclosedFrontmatter))?;
    let yaml = lines.get(1..close).unwrap_or_default().concat();
    let fields = fields::parse_fields(&yaml).map_err(|(line, kind)| {
        let line = line.map_or_else(|| field_line(&yaml, &kind), |line| line + 1);
        PageError::new(line, kind)
    })?;
    let head = lines.get(..=close).unwrap_or_default().concat();
    Ok((head, fields, close + 1))
}

fn field_line(yaml: &str, kind: &PageErrorKind) -> usize {
    let key = match kind {
        PageErrorKind::FieldNotScalar { key }
        | PageErrorKind::FieldKeyLength { key }
        | PageErrorKind::FieldValueTooLong { key } => key,
        PageErrorKind::UnclosedFrontmatter
        | PageErrorKind::Frontmatter { .. }
        | PageErrorKind::TimelineDate
        | PageErrorKind::TimelineSeparator
        | PageErrorKind::TimelineSource
        | PageErrorKind::TimelineText
        | PageErrorKind::TimelineBlankLine => return 2,
    };
    yaml.lines()
        .position(|line| {
            line.strip_prefix(key.as_str())
                .is_some_and(|rest| rest.starts_with(':'))
        })
        .map_or(2, |index| index + 2)
}

fn parse_entries(raw: &str, first_line: usize) -> Result<Vec<TimelineEntry>, PageError> {
    let lines: Vec<&str> = raw.lines().collect();
    let last_content = lines.iter().rposition(|line| !line.trim().is_empty());
    let mut entries = Vec::new();
    for (offset, line) in lines.iter().enumerate() {
        let number = first_line + offset;
        if line.trim().is_empty() {
            if last_content.is_some_and(|last| offset < last) {
                return Err(PageError::new(number, PageErrorKind::TimelineBlankLine));
            }
            continue;
        }
        entries.push(TimelineEntry::parse(line).map_err(|kind| PageError::new(number, kind))?);
    }
    Ok(entries)
}
