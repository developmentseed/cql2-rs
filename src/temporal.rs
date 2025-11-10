use crate::{Error, Expr};
use jiff::{SignedDuration, Timestamp};

const DAY: SignedDuration = SignedDuration::from_hours(24);
const SHYOFADAY: SignedDuration = DAY.checked_sub(SignedDuration::from_nanos(1)).unwrap();

fn strip_quotes(s: String) -> String {
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_string()
    } else {
        s
    }
}

fn parse_ts(s: &str) -> Result<Timestamp, Error> {
    let stripped = strip_quotes(s.to_string()).replace(' ', "T");
    let fromshort: String = match stripped.len() {
        4 => format!("{stripped}-01-01T00:00:00Z"),
        7 => format!("{stripped}-01T00:00:00Z"),
        10 => format!("{stripped}T00:00:00Z"),
        13 => format!("{stripped}:00:00Z"),
        16 => format!("{stripped}:00Z"),
        19 => format!("{stripped}Z"),
        _ => stripped,
    };

    fromshort.to_string().parse().map_err(Error::ParseTimestamp)
}

/// Struct to hold a range of timestamps.
#[derive(Debug, Clone)]
pub struct DateRange {
    /// Start timestamp of the range
    pub start: Timestamp,
    /// End timestamp of the range
    pub end: Timestamp,
}

impl TryFrom<Expr> for DateRange {
    type Error = Error;
    fn try_from(v: Expr) -> Result<DateRange, Error> {
        match v {
            Expr::Interval { interval } => {
                let start: Timestamp = parse_ts(&interval[0].to_text()?)?;
                let end: Timestamp = parse_ts(&interval[1].to_text()?)?;
                Ok(DateRange { start, end })
            }
            Expr::Timestamp { timestamp } => {
                let start: Timestamp = parse_ts(&timestamp.to_text()?)?;
                Ok(DateRange { start, end: start })
            }
            Expr::Date { date } => {
                let start: Timestamp = parse_ts(&date.to_text()?)?;
                let end: Timestamp = start + SHYOFADAY;
                Ok(DateRange { start, end })
            }
            Expr::Literal(v) => {
                let start: Timestamp = parse_ts(&v)?;
                Ok(DateRange { start, end: start })
            }
            _ => Err(Error::ExprToDateRange(v)),
        }
    }
}
// Implement PartialEq and PartialOrd for DateRange based on start and end boundaries
use std::cmp::Ordering;
/// Two DateRanges are equal if both their start and end timestamps match exactly.
impl PartialEq for DateRange {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}
/// Ordering for DateRanges:
/// - Less if this range ends before the other range starts.
/// - Greater if this range starts after the other range ends.
/// - Equal if boundaries match exactly.
/// - None if ranges overlap without boundary precedence.
impl PartialOrd for DateRange {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else if self.end < other.start {
            Some(Ordering::Less)
        } else if self.start > other.end {
            Some(Ordering::Greater)
        } else {
            None
        }
    }
}

/// Run a temporal operation.
pub fn temporal_op(left_expr: Expr, right_expr: Expr, op: &str) -> Result<Expr, Error> {
    let invop = match op {
        "t_after" => "t_before",
        "t_metby" => "t_meets",
        "t_overlappedby" => "t_overlaps",
        "t_startedby" => "t_starts",
        "t_contains" => "t_during",
        "t_finishedby" => "t_finishes",
        _ => op,
    };

    let left: DateRange;
    let right: DateRange;
    if invop == op {
        left = DateRange::try_from(left_expr)?;
        right = DateRange::try_from(right_expr)?;
    } else {
        right = DateRange::try_from(left_expr)?;
        left = DateRange::try_from(right_expr)?;
    }
    let out = match invop {
        "t_before" => Ok(left.end < right.start),
        "t_meets" => Ok(left.end == right.start),
        "t_overlaps" => {
            Ok(left.start < right.end && right.start < left.end && left.end < right.end)
        }
        "t_starts" => Ok(left.start == right.start && left.end < right.end),
        "t_during" => Ok(left.start > right.start && left.end < right.end),
        "t_finishes" => Ok(left.start > right.start && left.end == right.end),
        "t_equals" => Ok(left.start == right.start && left.end == right.end),
        "t_disjoint" => Ok(!(left.start <= right.end && left.end >= right.start)),
        "t_intersects" | "anyinteracts" => Ok(left.start <= right.end && left.end >= right.start),
        _ => Err(Error::OpNotImplemented("temporal")),
    };

    match out {
        Ok(v) => Ok(Expr::Bool(v)),
        _ => Err(Error::OperationError()),
    }
}

#[cfg(test)]
mod tests {
    use super::DateRange;
    use crate::Expr;
    use serde_json::json;

    #[test]
    fn timestamp_math() {
        // https://github.com/developmentseed/cql2-rs/issues/66
        let expr: Expr = serde_json::from_value(json!({"date": "2020-02-18"})).unwrap();
        let _: DateRange = expr.try_into().unwrap();
    }
}
