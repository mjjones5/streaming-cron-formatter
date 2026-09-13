use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FieldKind {
    Second,
    Minute,
    Hour,
    DayOfMonth,
    Month,
    DayOfWeek,
}

const FIVE_FIELD_SCHEDULE: [FieldKind; 5] = [
    FieldKind::Minute,
    FieldKind::Hour,
    FieldKind::DayOfMonth,
    FieldKind::Month,
    FieldKind::DayOfWeek,
];

const SIX_FIELD_SCHEDULE: [FieldKind; 6] = [
    FieldKind::Second,
    FieldKind::Minute,
    FieldKind::Hour,
    FieldKind::DayOfMonth,
    FieldKind::Month,
    FieldKind::DayOfWeek,
];

impl FieldKind {
    fn label(self) -> &'static str {
        match self {
            FieldKind::Second => "second",
            FieldKind::Minute => "minute",
            FieldKind::Hour => "hour",
            FieldKind::DayOfMonth => "day-of-month",
            FieldKind::Month => "month",
            FieldKind::DayOfWeek => "day-of-week",
        }
    }

    fn range(self) -> (i64, i64) {
        match self {
            FieldKind::Second => (0, 59),
            FieldKind::Minute => (0, 59),
            FieldKind::Hour => (0, 23),
            FieldKind::DayOfMonth => (1, 31),
            FieldKind::Month => (1, 12),
            // 0 and 7 both mean Sunday in most cron implementations.
            FieldKind::DayOfWeek => (0, 7),
        }
    }

    fn names(self) -> Option<&'static [&'static str]> {
        match self {
            FieldKind::Month => Some(&[
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ]),
            FieldKind::DayOfWeek => Some(&["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum NormalizeError {
    TooFewFields {
        found: usize,
    },
    NotANumber {
        field: &'static str,
        value: String,
    },
    OutOfRange {
        field: &'static str,
        value: String,
        lo: i64,
        hi: i64,
    },
    UnknownName {
        field: &'static str,
        value: String,
    },
    InvalidStep {
        field: &'static str,
        value: String,
    },
}

impl fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NormalizeError::TooFewFields { found } => {
                write!(f, "expected at least 5 schedule fields, found {found}")
            }
            NormalizeError::NotANumber { field, value } => {
                write!(f, "{field} field: '{value}' is not a number or name")
            }
            NormalizeError::OutOfRange {
                field,
                value,
                lo,
                hi,
            } => {
                write!(f, "{field} field: '{value}' is outside the range {lo}-{hi}")
            }
            NormalizeError::UnknownName { field, value } => {
                write!(f, "{field} field: '{value}' is not a recognized name")
            }
            NormalizeError::InvalidStep { field, value } => {
                write!(f, "{field} field: step '{value}' must be a positive integer")
            }
        }
    }
}

/// Normalizes a single crontab-style line, optionally followed by a
/// command. Blank lines and comment lines are returned trimmed but
/// otherwise untouched.
///
/// Standard crontabs use 5 schedule fields (minute hour day month
/// weekday). Some cron variants add a leading seconds field, making 6.
/// There's no marker in the line itself that says which form is meant, so
/// when there are at least 6 tokens we try the 6-field reading first and
/// fall back to the 5-field reading if that doesn't parse as a valid
/// schedule.
pub fn normalize_line(line: &str) -> Result<String, NormalizeError> {
    let trimmed = line.trim();

    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(trimmed.to_string());
    }

    let tokens: Vec<&str> = trimmed.split_whitespace().collect();

    if tokens.len() >= SIX_FIELD_SCHEDULE.len() {
        if let Ok(normalized) = normalize_with_schedule(&tokens, &SIX_FIELD_SCHEDULE) {
            return Ok(normalized);
        }
    }

    normalize_with_schedule(&tokens, &FIVE_FIELD_SCHEDULE)
}

fn normalize_with_schedule(
    tokens: &[&str],
    fields: &[FieldKind],
) -> Result<String, NormalizeError> {
    if tokens.len() < fields.len() {
        return Err(NormalizeError::TooFewFields { found: tokens.len() });
    }

    let (schedule, command) = tokens.split_at(fields.len());
    let normalized_schedule: Vec<String> = schedule
        .iter()
        .zip(fields.iter().copied())
        .map(|(f, kind)| normalize_field(f, kind))
        .collect::<Result<_, _>>()?;

    if command.is_empty() {
        Ok(normalized_schedule.join(" "))
    } else {
        Ok(format!(
            "{} {}",
            normalized_schedule.join(" "),
            command.join(" ")
        ))
    }
}

// A field is a comma-separated list of parts: "1,3,5" or "mon-fri" or "*/15".
fn normalize_field(field: &str, kind: FieldKind) -> Result<String, NormalizeError> {
    field
        .split(',')
        .map(|part| normalize_part(part, kind))
        .collect::<Result<Vec<_>, _>>()
        .map(|parts| parts.join(","))
}

fn normalize_part(part: &str, kind: FieldKind) -> Result<String, NormalizeError> {
    let part = part.trim();
    let (base, step) = match part.split_once('/') {
        Some((b, s)) => (b, Some(s.trim())),
        None => (part, None),
    };

    let base_normalized = match base.split_once('-') {
        Some((lo, hi)) => format!("{}-{}", normalize_atom(lo, kind)?, normalize_atom(hi, kind)?),
        None => normalize_atom(base, kind)?,
    };

    match step {
        Some(s) => {
            let valid = s.parse::<i64>().is_ok_and(|n| n > 0);
            if !valid {
                return Err(NormalizeError::InvalidStep {
                    field: kind.label(),
                    value: s.to_string(),
                });
            }
            Ok(format!("{base_normalized}/{s}"))
        }
        None => Ok(base_normalized),
    }
}

// Numbers get checked against the legal range for their position. Names
// (JAN, mon, Tue) get folded to a single canonical case and checked against
// the names allowed for that position, so "MON" and "mon" format the same
// way and a weekday name can't sneak into the month field.
fn normalize_atom(atom: &str, kind: FieldKind) -> Result<String, NormalizeError> {
    let atom = atom.trim();

    if atom == "*" {
        return Ok(atom.to_string());
    }

    let starts_alpha = atom.chars().next().is_some_and(|c| c.is_ascii_alphabetic());

    if starts_alpha {
        let names = kind.names().unwrap_or(&[]);
        return names
            .iter()
            .find(|name| name.eq_ignore_ascii_case(atom))
            .map(|name| name.to_string())
            .ok_or_else(|| NormalizeError::UnknownName {
                field: kind.label(),
                value: atom.to_string(),
            });
    }

    let value: i64 = atom.parse().map_err(|_| NormalizeError::NotANumber {
        field: kind.label(),
        value: atom.to_string(),
    })?;

    let (lo, hi) = kind.range();
    if value < lo || value > hi {
        return Err(NormalizeError::OutOfRange {
            field: kind.label(),
            value: atom.to_string(),
            lo,
            hi,
        });
    }

    Ok(atom.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapses_extra_whitespace() {
        assert_eq!(
            normalize_line("*  *\t* *   *   echo hi").unwrap(),
            "* * * * * echo hi"
        );
    }

    #[test]
    fn normalizes_lists_and_ranges() {
        assert_eq!(normalize_line("1, 3,5 * * * *").unwrap(), "1,3,5 * * * *");
        assert_eq!(normalize_line("* * * * MON-FRI").unwrap(), "* * * * Mon-Fri");
    }

    #[test]
    fn normalizes_step_values() {
        assert_eq!(normalize_line("*/15 0-6/2 * * *").unwrap(), "*/15 0-6/2 * * *");
    }

    #[test]
    fn passes_through_comments_and_blank_lines() {
        assert_eq!(normalize_line("  # daily backup  ").unwrap(), "# daily backup");
        assert_eq!(normalize_line("   ").unwrap(), "");
    }

    #[test]
    fn rejects_too_few_fields() {
        assert!(matches!(
            normalize_line("* * *"),
            Err(NormalizeError::TooFewFields { found: 3 })
        ));
    }

    #[test]
    fn accepts_values_at_the_edges_of_each_range() {
        assert_eq!(
            normalize_line("0 23 31 12 7 run").unwrap(),
            "0 23 31 12 7 run"
        );
        assert_eq!(
            normalize_line("59 0 1 1 0 run").unwrap(),
            "59 0 1 1 0 run"
        );
    }

    #[test]
    fn rejects_minute_out_of_range() {
        assert!(matches!(
            normalize_line("60 * * * *"),
            Err(NormalizeError::OutOfRange {
                field: "minute",
                lo: 0,
                hi: 59,
                ..
            })
        ));
    }

    #[test]
    fn rejects_hour_out_of_range() {
        assert!(matches!(
            normalize_line("* 24 * * *"),
            Err(NormalizeError::OutOfRange { field: "hour", .. })
        ));
    }

    #[test]
    fn rejects_day_of_month_zero() {
        assert!(matches!(
            normalize_line("* * 0 * *"),
            Err(NormalizeError::OutOfRange {
                field: "day-of-month",
                ..
            })
        ));
    }

    #[test]
    fn rejects_month_out_of_range_and_bad_names() {
        assert!(matches!(
            normalize_line("* * * 13 *"),
            Err(NormalizeError::OutOfRange { field: "month", .. })
        ));
        assert!(matches!(
            normalize_line("* * * Mon *"),
            Err(NormalizeError::UnknownName { field: "month", .. })
        ));
    }

    #[test]
    fn rejects_weekday_name_in_wrong_position() {
        assert!(matches!(
            normalize_line("* * * * Jan"),
            Err(NormalizeError::UnknownName {
                field: "day-of-week",
                ..
            })
        ));
    }

    #[test]
    fn rejects_non_numeric_field() {
        assert!(matches!(
            normalize_line("abc * * * *"),
            Err(NormalizeError::NotANumber { field: "minute", .. })
        ));
    }

    #[test]
    fn rejects_zero_step() {
        assert!(matches!(
            normalize_line("*/0 * * * *"),
            Err(NormalizeError::InvalidStep { field: "minute", .. })
        ));
    }

    #[test]
    fn normalizes_six_field_form_with_seconds() {
        assert_eq!(
            normalize_line("*/30 0 12 1 1 * backup").unwrap(),
            "*/30 0 12 1 1 * backup"
        );
        assert_eq!(
            normalize_line("59 59 23 31 DEC sat run").unwrap(),
            "59 59 23 31 Dec Sat run"
        );
    }

    #[test]
    fn falls_back_to_five_field_form_when_six_field_reading_is_invalid() {
        // The seconds slot would put "31" in the hour position (max 23),
        // so this can only be a valid schedule as five fields plus a command.
        assert_eq!(
            normalize_line("0 23 31 12 7 run").unwrap(),
            "0 23 31 12 7 run"
        );
    }

    #[test]
    fn reports_five_field_error_when_neither_reading_is_valid() {
        // Six tokens that are invalid both as a 6-field schedule and as a
        // 5-field schedule plus command: the error that surfaces is from
        // the 5-field fallback, since that's the reading that's checked
        // last.
        assert!(matches!(
            normalize_line("60 60 60 60 60 60"),
            Err(NormalizeError::OutOfRange {
                field: "minute",
                lo: 0,
                hi: 59,
                ..
            })
        ));
    }

    #[test]
    fn rejects_out_of_range_value_within_a_range_or_list() {
        assert!(matches!(
            normalize_line("0-60 * * * *"),
            Err(NormalizeError::OutOfRange { field: "minute", .. })
        ));
        assert!(matches!(
            normalize_line("1,99,5 * * * *"),
            Err(NormalizeError::OutOfRange { field: "minute", .. })
        ));
    }
}
