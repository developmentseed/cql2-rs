use crate::{Error, Expr};
use jiff::{Timestamp, ToSpan};

/// Struct to hold a range of timestamps.
#[derive(Debug, Clone, PartialEq)]
pub struct DateRange {
    start: Timestamp,
    end: Timestamp,
}

impl TryFrom<Expr> for DateRange {
    type Error = Error;
    fn try_from(v: Expr) -> Result<DateRange, Error> {
        match v {
            Expr::Interval { interval } => {
                let start_str: String = interval[0].to_text()?;
                let end_str: String = interval[1].to_text()?;
                let start: Timestamp = start_str.parse().unwrap();
                let end: Timestamp = end_str.parse().unwrap();
                Ok(DateRange { start, end })
            }
            Expr::Timestamp { timestamp } => {
                let start_str: String = timestamp.to_text()?;
                let start: Timestamp = start_str.parse().unwrap();
                Ok(DateRange { start, end: start })
            }
            Expr::Date { date } => {
                let start_str: String = date.to_text()?;
                let start: Timestamp = start_str.parse().unwrap();
                let end: Timestamp = start + 1.day() - 1.nanosecond();
                Ok(DateRange { start, end })
            }
            Expr::Literal(v) => {
                let start: Timestamp = v.parse().unwrap();
                Ok(DateRange { start, end: start })
            }
            _ => Err(Error::ExprToDateRange(v)),
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
    if invop != op {
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
