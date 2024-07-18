use pest::iterators::{Pairs, Pair};
use pest::pratt_parser::PrattParser;
use pest::Parser;
use serde_json;
use serde_derive::{Serialize, Deserialize};
use std::fs;
use boon::{Schemas, Compiler, SchemaIndex};

pub struct Validator {
    schemas: Schemas,
    index: SchemaIndex
}

impl Validator {
    pub fn new() -> Validator{
        let mut schemas = Schemas::new();
        let mut compiler = Compiler::new();
        let schema_json = serde_json::from_str(include_str!("../ogcapi-features/cql2/standard/schema/cql2.json")).expect("Could not parse schema to json");
        compiler.add_resource("/tmp/cql2.json", schema_json).expect("Could not add schema to compiler");
        let index = compiler.compile(
            "/tmp/cql2.json",
            &mut schemas
        ).expect("Could not compile schema");
        Validator{schemas,index}

    }
    pub fn validate(self, obj: serde_json::Value) -> bool {
        let valid = self.schemas.validate(&obj, self.index);
        match valid {
            Ok(()) => true,
            Err(e) => {
                let debug_level: &str = &std::env::var("CQL2_DEBUG_LEVEL").unwrap_or("1".to_string());
                match debug_level {
                    "3" => { println!("-----------\n{e:#?}\n---------------")},
                    "2" => { println!("-----------\n{e:?}\n---------------")},
                    "1" => { println!("-----------\n{e}\n---------------")},
                    _ => { println!("-----------\nCQL2 Is Invalid!\n---------------")},
                }

                false
            }
        }
    }
    pub fn validate_str(self, obj: &str) -> bool{
        self.validate(serde_json::from_str(obj).expect("Could not convert string to json."))
    }
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
    Geometry(GeomSteps),
    ArithValue(u64),
    FloatValue(f64),
    LiteralValue(String),
    BoolConst(bool),
    Property {
        property: String,
    },
    ArrayValue(Vec<Box<Expr>>),
    Coord(Vec<Box<f64>>),
    PCoordList(Vec<Expr>),
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
        Validator::new().validate_str(&self.as_json())
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


#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum GeomSteps {
    Geometry{
        r#type: String,
        coordinates: Box<GeomSteps>
    },
    Coord(Vec<GeomSteps>),
    CoordList(Vec<GeomSteps>),
    Ord(f64),
}

pub fn geom_type_str(t: &str) -> String {
    let out = match t.to_lowercase().as_str() {
        "point" => "Point",
        "linestring" => "LineString",
        "polygon" => "Polygon",
        "multipoint" => "MultiPoint",
        "multilinestring" => "MultiLineString",
        "multipolygon" => "MultiPolygon",
        "geometrycollection" => "GeometryCollection",
        _ => unreachable!("Invalid Geometry Type")
    };
    out.to_string()
}

fn parse_geom(p: Pair<Rule>) -> GeomSteps{
    match p.as_rule(){
        Rule::GEOMETRY => {
            let mut geom_pairs = p.into_inner().next().unwrap().into_inner();
            let r#type = geom_type_str(geom_pairs.next().unwrap().as_str());
            // let mut coordinates: Vec<GeomSteps> = Vec::new();
            // for pair in geom_pairs{
            //     let vals = parse_geom(pair);
            //     coordinates.push(vals);
            // }
            let coordinates = parse_geom(geom_pairs.next().unwrap());
            return GeomSteps::Geometry{
                r#type,
                coordinates: Box::new(coordinates)
            }
        },
        Rule::PCOORDLISTLISTLIST | Rule::PCOORDLISTLIST  | Rule::PCOORDLIST => {
            //println!("Rule: {:#?}", p.as_rule());
            let pairs = p.into_inner();
            //println!("Pairs>> {:#?}", pairs);
            let mut arr = Vec::new();
            for pair in pairs{
                //println!("Pair>>{:#?}", pair);
                let val = parse_geom(pair);
                //println!("Val>> {:#?}", val);
                arr.push(val);
            }
            //println!("Arr>> {:#?}", arr);
            return GeomSteps::CoordList(arr)

        },
        Rule::COORD => {
            let pairs = p.into_inner();
            //println!("COORD Pairs {:#?}", pairs);
            let mut coords = Vec::new();
            for coord in pairs{
                //println!("COORD {:#?}", coord);
                coords.push(parse_geom(coord))
            }
            return GeomSteps::Coord(coords)
        },
        Rule::DECIMAL => {
            return GeomSteps::Ord(p.as_str().parse::<f64>().unwrap())
        },
        _ => {
            unreachable!("Cannot parse rule into geometry")
        }
    }

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
            Rule::GEOMETRY => {
                let geom = parse_geom(primary);
                Expr::Geometry(geom)

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
                //println!("ARRAY PAIRS {:#?}", pairs);
                let mut array_elements = Vec::new();
                for pair in pairs {
                    array_elements.push(Box::new(parse_expr(pair.into_inner())))
                }
                Expr::ArrayValue(array_elements)

            },

            rule => unreachable!("Expr::parse expected atomic rule, found {:?}", rule),
        })
        .map_infix(|lhs, op, rhs| {
            //println!("INFIX: {:#?} {} {:#?}", lhs, op, rhs);
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
