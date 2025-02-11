use pg_escape::quote_identifier;
use serde_yaml;
use crate::{expr::CQL2Expr, Error, Expr};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt, fs::File, iter::repeat};
use dyn_fmt::AsStrFormatExt;

#[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
#[serde(rename_all = "UPPERCASE")]
enum OpType{
    // OP combine arguments by being between then ie a + b
    OP,
    // FUNC combines arguments within parens ie s_intersects(a,b)
    #[default]
    FUNC
}


#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "UPPERCASE")]
enum Cast{
    STRING,
    FLOAT,
    TIMESTAMPTZ,
    STRINGARRAY,
    WKT,
    BOOL,
    DTRANGE
}

fn default_args() -> Option<usize> {
    Some(2)
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
struct OpIn{
    #[serde(default = "default_args")]
    args: Option<usize>,
    #[serde(default)]
    optype: OpType,
    template: Option<String>,
    cast: Option<Cast>,
}


#[derive(Serialize, Deserialize, PartialEq, Debug, Default)]
pub struct Op{
    name: String,
    #[serde(default = "default_args")]
    args: Option<usize>,
    #[serde(default)]
    optype: OpType,
    template: Option<String>,
    cast: Option<Cast>,
}

fn repeat_join(s: &str, j: &str, n: usize) -> String {
    repeat(s).take(n).collect::<Vec<&str>>().join(j)
}

fn pad(s: &str)-> String {
    format!(" {} ", s)
}

fn add_parens(s: String) -> String {
    if s.starts_with("(") && s.ends_with(")") {
        s
    } else {
        format!("({})", s)
    }
}

fn rm_parens(s: String) -> String {
    if s.starts_with("(") && s.ends_with(")") {
        s.strip_prefix("(").unwrap().strip_suffix(")").unwrap().to_string()
    } else {
        s
    }
}

impl Op {
    fn get_template(&self, nargs: Option<usize>, placeholder: Option<&str>) -> String {
        let placeholder = placeholder.unwrap_or("{}");
        let nargs: usize = self.args.unwrap_or(nargs.unwrap_or(2));
        let name = match self.name.as_str() {
            "and" => "AND",
            "or" => "OR",
            _ => self.name.as_str()
        };
        match &self.template {
            Some(t) => t.to_string(),
            _ => {
                match self.optype {
                    OpType::FUNC => format!("{}({})", quote_identifier(name), repeat_join(&placeholder, ", ", nargs)),
                    OpType::OP => format!("({})", repeat_join(placeholder, pad(name).as_str(), nargs))
                }
            }

        }
    }
}

/// Configuration for templating Operators
#[derive(Serialize, Deserialize, Debug)]
pub struct Ops (HashMap<String, Op>);


impl Ops {
    /// Get inner hashmap from Ops
    pub fn inner(&self) -> &HashMap<String, Op> {
        &self.0
    }
    /// Get number of defined Ops
    pub fn len(&self) -> usize {
        self.inner().len()
    }

    /// Get op from ops
    pub fn get(&self, k: &str) -> Option<&Op> {
        self.inner().get(k)
    }

    /// Create a default configuration for generating operator templates
    ///
    /// # Examples
    /// ```
    /// use cql2::Ops;
    ///
    /// let ops = Ops::new().unwrap();
    /// assert_eq!(ops.len(), 48);
    /// ```
    ///
    pub fn new() -> Result<Ops, Error> {
        Ops::from_yaml_file("src/operators.yaml".to_string())
    }

    /// Load Operator Config from YAML
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Ops;
    ///
    /// let ops = Ops::from_yaml_file("src/operators.yaml".to_string());
    ///
    /// assert_eq!(ops.len(), 48);
    ///
    /// ```
    pub fn from_yaml_file(path: String) -> Result<Ops, Error> {
        let f = File::open(path)?;
        let ops: HashMap<String, OpIn> = serde_yaml::from_reader(f)?;
        let mut o: HashMap<String, Op> = HashMap::new();
        for (key, val) in ops.into_iter() {
            let name = key.clone();
            let v: Op = Op{
                name,
                args: val.args,
                optype: val.optype,
                template: val.template,
                cast: val.cast
            };
            println!("{}",v.get_template(None, None));
            let _ = o.insert(key, v);
        }
        Ok(Ops(o))
    }
}

lazy_static::lazy_static! {
    static ref OPSCONF: Ops = {
        use crate::Ops;
        Ops::new().expect("Could not load ops config")
    };
}

/// Expression member for an Op
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpsExpr {
    pub op: String,
    pub args: Vec<Box<Expr>>,
}

impl CQL2 for OpsExpr {
    fn to_text(&self) -> Result<String, Error> {
        let op = match OPSCONF.get(&self.op){
            Some(o) => o,
            _=> &Op{name: self.op.clone(), args: Some(self.args.len()), optype: OpType::FUNC, template: None, cast: None}
        };

        let args: Vec<String> =  self.args.iter().map(|x| x.to_text().expect("could not convert to text")).collect();
        Ok(op.get_template(Some(args.len()), Some("{}")).format(args.iter()))
    }
}
