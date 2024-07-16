use pest::iterators::{Pairs, Pair};
use pest::pratt_parser::PrattParser;
use pest::Parser;
use serde_json;
use serde_derive::{Serialize, Deserialize};
use geo_types::Geometry;
use wkt::TryFromWkt;
use geojson::ser::serialize_geometry;
use geojson::de::deserialize_geometry;
use jsonschema::JSONSchema;
use jsonschema::Draft::Draft202012;
use std::fs;

pub fn get_validator()-> JSONSchema{
    JSONSchema::options()
        .with_draft(Draft202012)
        .should_validate_formats(false)
        .compile(
            &serde_json::from_str(include_str!("../ogcapi-features/cql2/standard/schema/cql2.json")).unwrap()
        )
        .expect("Invalid Schema")
}


#[derive(pest_derive::Parser)]
#[grammar = "cql2.pest"]
pub struct CQL2Parser;



#[derive(Debug, Serialize, Deserialize, Clone)]
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
    // #[serde(serialize_with = "serialize_geometry", deserialize_with = "deserialize_geometry")]
    Geometry{
        r#type: String,
        coordinates: Box<Expr>
    },
    ArithValue(u64),
    FloatValue(f64),
    LiteralValue(String),
    BoolConst(bool),
    Property {
        property: String,
    },
    ArrayValue(Vec<Box<Expr>>),
    Coord(Vec<Box<Expr>>),
    PCoordList(Vec<Box<Expr>>),
    PCoordListList(Vec<Box<Expr>>),
    PCoordListListList(Vec<Box<Expr>>),

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
    pub fn validate(&self) -> bool {
        let schema = get_validator();
        let json = serde_json::from_str(&self.as_json()).expect("Bad Json Returned");

        let result = schema.validate(&json);
        if let Err(errors) = result {
            for error in errors {
                println!("Validation error: {}", error);
                println!("Instance path: {}", error.instance_path.to_string());
            }
            return false
        }
        return true
    }
}

lazy_static::lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        use pest::pratt_parser::{Assoc::*, Op};
        use Rule::*;
        PrattParser::new()
            .op(Op::infix(Or, Left))
            .op(Op::infix(And, Left))
            .op(Op::prefix(UnaryNot))
            .op(Op::infix(Eq, Right))
            .op(
                Op::infix(NotEq, Right) |
                Op::infix(Gt, Right) |
                Op::infix(GtEq, Right) |
                Op::infix(Lt, Right) |
                Op::infix(LtEq, Right)
            )
            .op(Op::infix(Like, Right))
            .op(Op::infix(Between, Left))
            .op(Op::infix(In, Left))
            .op(Op::postfix(IsNullPostfix))
            .op(Op::infix(Is, Right))
            .op(
                Op::infix(Add, Left) |
                Op::infix(Subtract, Left)
            )
            .op(
                Op::infix(Multiply, Left) |
                Op::infix(Divide, Left) |
                Op::infix(Modulo, Left)
            )
            .op(Op::infix(Power, Left))
            .op(Op::prefix(Negative))
        };
}

pub fn normalize_op(op: &str) -> String {
    let oper = op.to_lowercase();
    let operator: &str = match oper.as_str() {
        "eq" => "=",
        _ => &oper,
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

pub fn parse_geom(rule: Pair<Rule>) -> Expr::Geometry{
    let mut pairs
}

fn parse_expr(expression_pairs: Pairs<'_, Rule>) -> Expr {
    PRATT_PARSER
        .map_primary(|primary| match primary.as_rule() {
            Rule::Expr | Rule::ExpressionInParentheses => parse_expr(primary.into_inner()),
            Rule::Unsigned => {
                let u64_value = primary.as_str().parse::<u64>().unwrap();
                Expr::ArithValue(u64_value)
            },
            Rule::DECIMAL => {
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
            Rule::COORD => {
                let pairs = primary.into_inner();
                println!("COORD Pairs {:#?}", pairs);
                let mut coords = Vec::new();
                for coord in pairs{
                    println!("COORD {:#?}", coord);
                    coords.push(Box::new(Expr::FloatValue(coord.as_str().parse::<f64>().unwrap())))
                }
                println!("COORDS {:#?}", coords);
                Expr::Coord(coords)
            },
            Rule::PCOORDLIST => {
                let pairs = primary.into_inner();
                println!("PCOORDLIST Pairs {:#?}", pairs);
                let mut coords = Vec::new();
                for coord in pairs{
                    println!("COORD {:#?}", coord);
                    coords.push(Box::new(Expr::Coord(parse_expr(coord))));
                }
                println!("COORDS {:#?}", coords);
                Expr::Coord(coords)
            },
            Rule::GEOMETRY => {
                let mut pairs = primary.into_inner().next().unwrap().into_inner();
                println!("GEOMETRY PAIRS {:#?}", pairs);
                let geom = pairs.next().unwrap();
                println!("GEOM {:#?}", geom);
                let geomtype = geom.as_str();

                let gnext = parse_expr(pairs);
                // let mut array_elements: Vec<Box<Expr>> = Vec::new();
                // for pair in gpairs{
                //     match pair.as_rule() {
                //         Rule::DECIMAL => {
                //             let num = pair.as_str().parse::<f64>().unwrap();
                //             array_elements.push(Box::new(Expr::FloatValue(num)));
                //         },
                //         rule => {
                //             println!("Unmatched {:#?}", rule);
                //         }

                //     }
                // }
                //let array_elements = parse_expr(gpairs);
                println!("gnext {:#?}", gnext);
                Expr::Geometry{
                    r#type: geomtype.to_string(),
                    coordinates: Box::new(gnext)
                }
            },
            Rule::Function => {
                let mut pairs = primary.into_inner();
                let op = strip_quotes(pairs.next().unwrap().as_str()).to_lowercase();
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
            Rule::Array => {
                let pairs = primary.into_inner();
                println!("ARRAY PAIRS {:#?}", pairs);
                let mut array_elements = Vec::new();
                for pair in pairs {
                    array_elements.push(Box::new(parse_expr(pair.into_inner())))
                }
                Expr::ArrayValue(array_elements)

            },

            rule => unreachable!("Expr::parse expected atomic rule, found {:?}", rule),
        })
        .map_infix(|lhs, op, rhs| {
            println!("INFIX: {:#?} {} {:#?}", lhs, op, rhs);
            let opstring = opstr(op);
            let origargs = vec![Box::new(lhs.clone()),Box::new(rhs.clone())];
            let rhsclone = rhs.clone();
            let retexpr = match lhs {
                Expr::Operation{ op, args} if op == "between".to_string() =>{
                    let mut lhsargs = args.into_iter();
                    Expr::Operation{
                        op,
                        args: vec![lhsargs.next().unwrap(), lhsargs.next().unwrap(), Box::new(rhsclone)]
                    }
                },
                _ => Expr::Operation {
                        op: opstring,
                        args: origargs
                    }
            };
            return retexpr


        })
        .map_prefix(|op, child| match op.as_rule() {
            Rule::UnaryNot => Expr::Operation { op: "not".to_string(), args: vec![Box::new(child)] } ,
            Rule::Negative => Expr::Operation { op: "*".to_string(), args: vec![Box::new(Expr::FloatValue(-1.0)),Box::new(child)] },
            rule => unreachable!("Expr::parse expected prefix operator, found {:?}", rule),
        })
        .map_postfix(|child, op| match op.as_rule() {
            Rule::IsNullPostfix => Expr::Operation { op: "isNull".to_string(), args: vec![Box::new(child)] } ,
            rule => unreachable!("Expr::parse expected postfix operator, found {:?}", rule),
        })
        .parse(expression_pairs)
}



pub fn parse(cql2: &str) -> Expr{
    if cql2.starts_with("{"){
        let expr: Expr = serde_json::from_str(cql2).unwrap();
        return expr;
    } else {
        let mut pairs = CQL2Parser::parse(Rule::Expr, cql2).unwrap();
        return parse_expr(pairs.next().unwrap().into_inner());
    }
}

pub fn parse_file(f: &str) -> Expr {
    let cql2 = fs::read_to_string(f).unwrap();
    return parse(&cql2)
}
