use crate::{Error, Expr};
use jiff::{Timestamp, ToSpan};

/// Struct to hold a range of timestamps.
#[derive(Debug,Clone,PartialEq)]
pub struct DateRange {
    start: Timestamp,
    end: Timestamp,
}

impl TryFrom<Expr> for DateRange {
    type Error = Error;
    fn try_from(v: Expr) -> Result<DateRange, Error> {

        match v {
            Expr::Interval{interval} => {
                let start_str: String = interval[0].to_text()?;
                let end_str: String = interval[1].to_text()?;
                let start: Timestamp = start_str.parse().unwrap();
                let end: Timestamp = end_str.parse().unwrap();
                Ok(DateRange{start, end})
            }
            Expr::Timestamp{timestamp} => {
                let start_str: String = timestamp.to_text()?;
                let start: Timestamp = start_str.parse().unwrap();
                Ok(DateRange{start, end: start})
            }
            Expr::Date{date} => {
                let start_str: String = date.to_text()?;
                let start: Timestamp = start_str.parse().unwrap();
                let end: Timestamp = start + 1.day() - 1.nanosecond();
                Ok(DateRange{start, end})
            }
            Expr::Literal(v) => {
                let start: Timestamp = v.parse().unwrap();
                Ok(DateRange{start, end: start})
            }
            _ => Err(Error::ExprToDateRange()),
        }
    }
}

/// Run a temporal operation.
pub fn temporal_op(left: &DateRange, right: &DateRange, op: &str) -> Result<bool, Error> {
    match op {
        "t_before" => Ok(left.end < right.start),
        "t_after" => temporal_op(right, left, "t_before"),
        "t_meets" => Ok(left.end == right.start),
        "t_metby" => temporal_op(right, left, "t_meets"),
        "t_overlaps" => {
            Ok(left.start < right.end && right.start < left.end && left.end < right.end)
        }
        "t_overlappedby" => temporal_op(right, left, "t_overlaps"),
        "t_starts" => Ok(left.start == right.start && left.end < right.end),
        "t_startedby" => temporal_op(right, left, "t_starts"),
        "t_during" => Ok(left.start > right.start && left.end < right.end),
        "t_contains" => temporal_op(right, left, "t_during"),
        "t_finishes" => Ok(left.start > right.start && left.end == right.end),
        "t_finishedby" => temporal_op(right, left, "t_finishes"),
        "t_equals" => Ok(left.start == right.start && left.end == right.end),
        "t_disjoint" => Ok(!(temporal_op(left, right, "t_intersects").unwrap())),
        "t_intersects" | "anyinteracts" => Ok(left.start <= right.end && left.end >= right.start),
        _ => Err(Error::OpNotImplemented("temporal")),
    }
}
