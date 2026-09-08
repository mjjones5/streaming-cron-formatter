use std::fmt;

#[derive(Debug)]
pub enum NormalizeError {
    TooFewFields { found: usize },
}

impl fmt::Display for NormalizeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NormalizeError::TooFewFields { found } => {
                write!(f, "expected at least 5 schedule fields, found {found}")
            }
        }
    }
}

/// Normalizes a single crontab-style line: minute hour day month weekday,
/// optionally followed by a command. Blank lines and comment lines are
/// returned trimmed but otherwise untouched.
pub fn normalize_line(line: &str) -> Result<String, NormalizeError> {
    let trimmed = line.trim();

    if trimmed.is_empty() || trimmed.starts_with('#') {
        return Ok(trimmed.to_string());
    }

    let mut tokens = trimmed.split_whitespace();
    let schedule: Vec<&str> = tokens.by_ref().take(5).collect();

    if schedule.len() < 5 {
        return Err(NormalizeError::TooFewFields {
            found: schedule.len(),
        });
    }

    let command: Vec<&str> = tokens.collect();
    let normalized_schedule: Vec<String> = schedule.iter().map(|f| normalize_field(f)).collect();

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
fn normalize_field(field: &str) -> String {
    field
        .split(',')
        .map(normalize_part)
        .collect::<Vec<_>>()
        .join(",")
}

fn normalize_part(part: &str) -> String {
    let part = part.trim();
    let (base, step) = match part.split_once('/') {
        Some((b, s)) => (b, Some(s.trim())),
        None => (part, None),
    };

    let base_normalized = match base.split_once('-') {
        Some((lo, hi)) => format!("{}-{}", normalize_atom(lo), normalize_atom(hi)),
        None => normalize_atom(base),
    };

    match step {
        Some(s) => format!("{base_normalized}/{s}"),
        None => base_normalized,
    }
}

// Numbers and "*" pass through unchanged. Names (JAN, mon, Tue) get folded
// to a single canonical case so "MON" and "mon" format the same way.
fn normalize_atom(atom: &str) -> String {
    let atom = atom.trim();
    let starts_alpha = atom.chars().next().is_some_and(|c| c.is_ascii_alphabetic());

    if !starts_alpha {
        return atom.to_string();
    }

    let mut chars = atom.chars();
    let first = chars.next().unwrap().to_ascii_uppercase();
    let rest: String = chars.map(|c| c.to_ascii_lowercase()).collect();
    format!("{first}{rest}")
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
        assert_eq!(normalize_line("* * * MON-FRI *").unwrap(), "* * * Mon-Fri *");
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
}
