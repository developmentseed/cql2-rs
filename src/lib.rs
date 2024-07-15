use pest::iterators::{Pairs, Pair};
use pest::pratt_parser::PrattParser;
use pest::Parser;
use serde_json;
use serde_derive::Serialize;
use geo_types::Geometry;
use wkt::TryFromWkt;
use geojson::ser::serialize_geometry;
use jsonschema::JSONSchema;
use std::fs;


#[derive(pest_derive::Parser)]
#[grammar = "cql2.pest"]
pub struct CQL2Parser;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Expr {
    Operation {
        op: String,
        args: Vec<Box<Expr>>,
    },
    Interval {
        interval: Vec<Box<Expr>>,
    },
    Timestamp {
        timestamp: Box<Expr>,
    },
    Date {
        date: Box<Expr>,
    },
    #[serde(serialize_with = "serialize_geometry")]
    Geometry(Geometry),
    ArithValue(u64),
    FloatValue(f64),
    LiteralValue(String),
    BoolConst(bool),
    Not {
        child: Box<Expr>,
    },
    IsNull {
        child: Box<Expr>,
    },
    Property {
        property: String,
    },
}

impl Expr {
    /* fn as_cql2_text(&self) -> String {
        return "cql2-text".to_string();
    }
    fn as_sql() -> String {
        return "sql".to_string();
    } */
    pub fn as_json(&self) -> String {
        return serde_json::to_string(&self).unwrap();
    }
    pub fn as_json_pretty(&self) -> String {
        return serde_json::to_string_pretty(&self).unwrap();
    }
    pub fn validate(&self) {
        let schema_data = fs::read_to_string("ogcapi-features/cql2/standard/schema/cql2.json").expect("Unable to read json schema");
        let schema_json: serde_json::Value = serde_json::from_str(&schema_data).expect("Unable to parse json schema");
        let schema = JSONSchema::compile(&schema_json).expect("Invalid Schema");

        let json = serde_json::from_str(&self.as_json()).expect("Bad Json Returned");

        let result = schema.validate(&json);
        if let Err(errors) = result {
            for error in errors {
                println!("Validation error: {}", error);
                println!("Instance path: {}", error.instance_path);
            }
        }
    }
}

lazy_static::lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        use pest::pratt_parser::{Assoc::*, Op};
        use Rule::*;

        // Precedence is defined lowest to highest
        PrattParser::new()
                .op(Op::infix(Or, Left))
                .op(Op::infix(Between, Left))
                .op(Op::infix(And, Left))
                .op(Op::prefix(UnaryNot))
                .op(
                    Op::infix(Eq, Right) | Op::infix(NotEq, Right) | Op::infix(NotEq, Right)
                        | Op::infix(Gt, Right) | Op::infix(GtEq, Right) | Op::infix(Lt, Right)
                        | Op::infix(LtEq, Right) | Op::infix(In, Right)
                )
                .op(Op::infix(Add, Left) | Op::infix(Subtract, Left))
                .op(Op::infix(Multiply, Left) | Op::infix(Divide, Left) | Op::infix(ConcatInfixOp, Left))
                .op(Op::postfix(IsNullPostfix))
        };
}

pub fn normalize_op(op: &str) -> String {
    let operator: &str = match op {
        "eq" => "=",
        _ => op,
    };
    return operator.to_string();
}

pub fn strip_quotes(quoted_string: &str) -> String {
    let len = quoted_string.len();
    let bytes = quoted_string.as_bytes();
    if
        (bytes[0] == b'"' && bytes[len-1] == b'"')
        || (bytes[0] == b'\'' && bytes[len-1] == b'\'') {
        Some(&quoted_string[1..len-1]).unwrap().to_string()
    }
    else {
        quoted_string.to_string()
    }
}

pub fn opstr(op: Pair<Rule>) -> String {
    return normalize_op(op.as_str());
}

fn parse_expr(expression_pairs: Pairs<'_, Rule>) -> Expr {
    PRATT_PARSER
        .map_primary(|primary| match primary.as_rule() {
            Rule::Expr | Rule::ExpressionInParentheses => parse_expr(primary.into_inner()),
            Rule::Unsigned => {
                let u64_value = primary.as_str().parse::<u64>().unwrap();
                Expr::ArithValue(u64_value)
            },
            Rule::Decimal => {
                let f64_value = primary.as_str().parse::<f64>().unwrap();
                Expr::FloatValue(f64_value)
            },
            Rule::SingleQuotedString => {
                Expr::LiteralValue(strip_quotes(primary.as_str()))
            },
            Rule::True | Rule::False => {
                let bool_value = primary.as_str().to_lowercase().parse::<bool>().unwrap();
                Expr::BoolConst(bool_value)
            },
            Rule::Identifier => {
                Expr::Property {
                    property: strip_quotes(primary.as_str()),
                }
            },
            Rule::GEOMETRY => {
                let wkt = primary.as_str();
                let geom = Geometry::try_from_wkt_str(wkt).unwrap();
                println!("{:#?}", primary);
                println!("{:#?}", geom);
                Expr::Geometry(geom)
            },
            Rule::Function => {
                let mut pairs = primary.into_inner();
                let op = strip_quotes(pairs.next().unwrap().as_str()).to_lowercase();

                println!("{:#?}", pairs);
                let mut args = Vec::new();
                for pair in pairs {
                    args.push(Box::new(parse_expr(pair.into_inner())))
                }
                match op.as_str() {
                    "interval" => Expr::Interval{interval: args},
                    "date" => Expr::Date{date: args.into_iter().nth(0).unwrap()},
                    "timestamp" => Expr::Timestamp{timestamp: args.into_iter().nth(0).unwrap()},
                    _ => Expr::Operation{ op:op, args:args },
                }
            },

            rule => unreachable!("Expr::parse expected atomic rule, found {:?}", rule),
        })
        .map_infix(|lhs, op, rhs| {
            Expr::Operation {
                op: opstr(op),
                args: vec![Box::new(lhs), Box::new(rhs)],
            }
        })
        .map_prefix(|op, child| match op.as_rule() {
            Rule::UnaryNot => Expr::Not {
                child: Box::new(child),
            },
            rule => unreachable!("Expr::parse expected prefix operator, found {:?}", rule),
        })
        .map_postfix(|child, op| match op.as_rule() {
            Rule::IsNullPostfix => Expr::IsNull {
                child: Box::new(child),
            },
            rule => unreachable!("Expr::parse expected postfix operator, found {:?}", rule),
        })
        .parse(expression_pairs)
}

pub fn parse(cql2: &str) -> Expr{
    let mut pairs = CQL2Parser::parse(Rule::Expr, cql2).unwrap();
    return parse_expr(pairs.next().unwrap().into_inner());
}
