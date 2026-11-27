//! Reads the records of a holiday file.

use super::calendar::{Holidays, PublicHoliday, SchoolHoliday};

/// The kind of record a keyword opens.
#[derive(Debug, Clone, Copy)]
enum Record {
    /// `[holiday]`: a date and a name.
    Public,
    /// `[holidays]`: a start date, an end date and a name.
    School,
}

impl Record {
    /// How many field lines the record takes; anything after them is a trailing comment.
    fn arity(self) -> usize {
        match self {
            Self::Public => 2,
            Self::School => 3,
        }
    }
}

/// `yyyymmdd` as a number. `None` unless it has eight digits, a month of `1..=12` and a day of
/// `1..=31` (the day is not checked against the month).
#[must_use]
pub fn parse_date(s: &str) -> Option<u32> {
    if s.len() != 8 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let date: u32 = s.parse().ok()?;
    let (m, d) = (date / 100 % 100, date % 100);
    ((1..=12).contains(&m) && (1..=31).contains(&d)).then_some(date)
}

impl Holidays {
    /// Parses the decoded text of a holiday file. Records with a malformed date are skipped;
    /// lines outside a record and the `....` and `----` comment lines are ignored.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut out = Self::default();
        let mut record: Option<Record> = None;
        let mut fields: Vec<&str> = Vec::new();
        for line in text.lines().map(str::trim) {
            if let Some(keyword) = line.strip_prefix('[') {
                record = match keyword.trim_end_matches(']').to_ascii_lowercase().as_str() {
                    "holiday" => Some(Record::Public),
                    "holidays" => Some(Record::School),
                    _ => None,
                };
                fields.clear();
                continue;
            }
            let Some(kind) = record else {
                continue;
            };
            if line.is_empty() || line.starts_with("....") || line.starts_with("---") {
                continue;
            }
            fields.push(line);
            if fields.len() == kind.arity() {
                out.push(kind, &fields);
                record = None;
            }
        }
        out
    }

    /// Adds a complete record (its fields are checked here).
    fn push(&mut self, kind: Record, fields: &[&str]) {
        match kind {
            Record::Public => {
                if let Some(date) = parse_date(fields[0]) {
                    self.public.push(PublicHoliday {
                        date,
                        name: fields[1].to_string(),
                    });
                }
            }
            Record::School => {
                if let (Some(start), Some(end)) = (parse_date(fields[0]), parse_date(fields[1])) {
                    self.school.push(SchoolHoliday {
                        start,
                        end,
                        name: fields[2].to_string(),
                    });
                }
            }
        }
    }
}
