use boon::{Compiler, SchemaIndex, Schemas};
use geozero::geojson::GeoJsonWriter;
use geozero::wkt::Wkt;
use geozero::{CoordDimensions, GeozeroGeometry, ToJson};
use pest::iterators::{Pair, Pairs};
use pest::pratt_parser::PrattParser;
use pest::Parser;
use serde_derive::{Deserialize, Serialize};
use std::fs;
pub struct Validator {
    schemas: Schemas,
    index: SchemaIndex,
}

impl Validator {
    pub fn new() -> Validator {
        let mut schemas = Schemas::new();
        let mut compiler = Compiler::new();
        let schema_json = serde_json::from_str(include_str!("cql2.json"))
            .expect("Could not parse schema to json");
        compiler
            .add_resource("/tmp/cql2.json", schema_json)
            .expect("Could not add schema to compiler");
        let index = compiler
            .compile("/tmp/cql2.json", &mut schemas)
            .expect("Could not compile schema");
        Validator { schemas, index }
    }

    pub fn validate(self, obj: serde_json::Value) -> bool {
        let valid = self.schemas.validate(&obj, self.index);
        match valid {
            Ok(()) => true,
            Err(e) => {
                let debug_level: &str =
                    &std::env::var("CQL2_DEBUG_LEVEL").unwrap_or("1".to_string());
                match debug_level {
                    "3" => {
                        println!("-----------\n{e:#?}\n---------------")
                    }
                    "2" => {
                        println!("-----------\n{e:?}\n---------------")
                    }
                    "1" => {
                        println!("-----------\n{e}\n---------------")
                    }
                    _ => {
                        println!("-----------\nCQL2 Is Invalid!\n---------------")
                    }
                }

                false
            }
        }
    }
    pub fn validate_str(self, obj: &str) -> bool {
        self.validate(serde_json::from_str(obj).expect("Could not convert string to json."))
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(pest_derive::Parser)]
#[grammar = "cql2.pest"]
pub struct CQL2Parser;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Expr {
    Operation { op: String, args: Vec<Box<Expr>> },
    Interval { interval: Vec<Box<Expr>> },
    Timestamp { timestamp: Box<Expr> },
    Date { date: Box<Expr> },
    Geometry(serde_json::Value),
    Float(f64),
    Literal(String),
    Bool(bool),
    Property { property: String },
    Array(Vec<Box<Expr>>),
}

impl Expr {
    /* fn as_cql2_text(&self) -> String {
        return "cql2-text".to_string();
    }
    fn as_sql() -> String {
        return "sql".to_string();
    } */
    pub fn as_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
    pub fn as_json_pretty(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap()
    }
    pub fn validate(&self) -> bool {
        Validator::new().validate_str(&self.as_json())
    }
}

lazy_static::lazy_static! {
    static ref PRATT_PARSER: PrattParser<Rule> = {
        use pest::pratt_parser::{Assoc::*, Op};
        use Rule::*;
        PrattParser::new()
            .op(Op::infix(Or, Left))
            .op(Op::infix(Between, Left))
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
    operator.to_string()
}

pub fn strip_quotes(quoted_string: &str) -> String {
    let len = quoted_string.len();
    let bytes = quoted_string.as_bytes();
    if (bytes[0] == b'"' && bytes[len - 1] == b'"')
        || (bytes[0] == b'\'' && bytes[len - 1] == b'\'')
    {
        quoted_string[1..len - 1].to_string()
    } else {
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
            Rule::Unsigned => Expr::Float(
                primary
                    .as_str()
                    .parse::<f64>()
                    .expect("Could not cast value to float"),
            ),
            Rule::DECIMAL => Expr::Float(
                primary
                    .as_str()
                    .parse::<f64>()
                    .expect("Could not cast value to float"),
            ),
            Rule::SingleQuotedString => Expr::Literal(strip_quotes(primary.as_str())),
            Rule::True | Rule::False => {
                let bool_value = primary.as_str().to_lowercase().parse::<bool>().unwrap();
                Expr::Bool(bool_value)
            }
            Rule::Identifier => Expr::Property {
                property: strip_quotes(primary.as_str()),
            },
            Rule::GEOMETRY => {
                let geom_wkt = Wkt(primary.as_str());
                let mut out: Vec<u8> = Vec::new();
                let mut p = GeoJsonWriter::with_dims(&mut out, CoordDimensions::xyz());
                let _ = geom_wkt.process_geom(&mut p);
                Expr::Geometry(serde_json::from_str(&geom_wkt.to_json().unwrap()).unwrap())
            }
            Rule::Function => {
                let mut pairs = primary.into_inner();
                let op = strip_quotes(pairs.next().unwrap().as_str()).to_lowercase();
                let mut args = Vec::new();
                for pair in pairs {
                    args.push(Box::new(parse_expr(pair.into_inner())))
                }
                match op.as_str() {
                    "interval" => Expr::Interval { interval: args },
                    "date" => Expr::Date {
                        date: args.into_iter().next().unwrap(),
                    },
                    "timestamp" => Expr::Timestamp {
                        timestamp: args.into_iter().next().unwrap(),
                    },
                    _ => Expr::Operation { op, args },
                }
            }
            Rule::Array => {
                let pairs = primary.into_inner();
                let mut array_elements = Vec::new();
                for pair in pairs {
                    array_elements.push(Box::new(parse_expr(pair.into_inner())))
                }
                Expr::Array(array_elements)
            }

            rule => unreachable!("Expr::parse expected atomic rule, found {:?}", rule),
        })
        .map_infix(|lhs, op, rhs| {
            let mut opstring = opstr(op);

            let mut notflag: bool = false;
            if opstring.starts_with("not") {
                opstring = opstring.replace("not ", "");
                notflag = true;
            }

            let origargs = vec![Box::new(lhs.clone()), Box::new(rhs.clone())];
            let mut retexpr;
            let mut lhsclone = lhs.clone();
            let rhsclone = rhs.clone();

            let mut outargs: Vec<Box<Expr>> = Vec::new();

            match lhsclone {
                Expr::Operation { ref op, ref args } if *op == "and" => {
                    for arg in args.iter() {
                        outargs.push(arg.clone());
                    }
                    outargs.push(Box::new(rhsclone));
                    return Expr::Operation {
                        op: "and".to_string(),
                        args: outargs,
                    };
                }
                _ => (),
            }

            if opstring == "between" {
                match lhsclone {
                    Expr::Operation { op, args } if op == "not" => {
                        let mut lhsargs = args.into_iter();
                        lhsclone = *lhsargs.next().unwrap();
                        notflag = true;
                    }
                    _ => (),
                }

                match rhs {
                    Expr::Operation { op, args } if op == "and" => {
                        let mut rhsargs = args.into_iter();
                        retexpr = Expr::Operation {
                            op: opstring,
                            args: vec![
                                Box::new(lhsclone),
                                rhsargs.next().unwrap(),
                                rhsargs.next().unwrap(),
                            ],
                        };
                        if rhsargs.len() >= 1 {
                            let mut newargs = vec![Box::new(retexpr)];
                            for rhsarg in rhsargs {
                                newargs.push(rhsarg);
                            }
                            retexpr = Expr::Operation {
                                op: "and".to_string(),
                                args: newargs,
                            };
                        }
                    }
                    _ => unreachable!("RHS of between must be And Statement"),
                }
            } else {
                retexpr = Expr::Operation {
                    op: opstring,
                    args: origargs,
                };
            }

            if notflag {
                return Expr::Operation {
                    op: "not".to_string(),
                    args: vec![Box::new(retexpr)],
                };
            }
            retexpr
        })
        .map_prefix(|op, child| match op.as_rule() {
            Rule::UnaryNot => Expr::Operation {
                op: "not".to_string(),
                args: vec![Box::new(child)],
            },
            Rule::Negative => Expr::Operation {
                op: "*".to_string(),
                args: vec![Box::new(Expr::Float(-1.0)), Box::new(child)],
            },
            rule => unreachable!("Expr::parse expected prefix operator, found {:?}", rule),
        })
        .map_postfix(|child, op| match op.as_rule() {
            Rule::IsNullPostfix => Expr::Operation {
                op: "isNull".to_string(),
                args: vec![Box::new(child)],
            },
            rule => unreachable!("Expr::parse expected postfix operator, found {:?}", rule),
        })
        .parse(expression_pairs)
}

pub fn parse(cql2: &str) -> Expr {
    if cql2.starts_with('{') {
        let expr: Expr = serde_json::from_str(cql2).unwrap();
        expr
    } else {
        let mut pairs = CQL2Parser::parse(Rule::Expr, cql2).unwrap();
        return parse_expr(pairs.next().unwrap().into_inner());
    }
}

pub fn parse_file(f: &str) -> Expr {
    let cql2 = fs::read_to_string(f).unwrap();
    parse(&cql2)
}
