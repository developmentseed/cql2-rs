use crate::ToSqlAst;
use crate::Error;
use crate::Expr;

/// Traits for generating SQL for DuckDB with Spatial Extension
pub trait ToDuckSQL {
    /// Convert Expression to SQL for DuckDB with Spatial Extension
    fn to_ducksql(&self) -> Result<String, Error>;
}

impl ToDuckSQL for Expr {
    /// Converts this expression to DuckDB SQL.
    /// WARNING: This is an experimental feature with limited tests subject to change!
    ///
    /// # Examples
    ///
    /// ```
    /// use cql2::Expr;
    /// use cql2::ToDuckSQL;
    ///
    /// let expr = Expr::Bool(true);
    /// assert_eq!(expr.to_ducksql().unwrap(), "true");
    /// ```
    /// ```
    /// use cql2::Expr;
    /// use cql2::ToDuckSQL;
    ///
    /// let expr: Expr = "s_intersects(geom, POINT(0 0)) and foo >= 1 and bar='baz' and TIMESTAMP('2020-01-01 00:00:00Z') >= BoRk and DATE('2020-01-01') > b and q = 'hello World!') > b".parse().unwrap();
    /// assert_eq!(expr.to_ducksql().unwrap(), "st_intersects(geom, st_geomfromtext('POINT(0 0)')) AND foo >= 1 AND bar = 'baz' AND CAST('2020-01-01 00:00:00Z' AS TIMESTAMP WITH TIME ZONE) >= \"BoRk\" AND CAST('2020-01-01' AS DATE) > b AND q = 'hello World!'");
    /// ```
    /// ```
    /// use cql2::Expr;
    /// use cql2::ToDuckSQL;
    ///
    /// let expr: Expr = "t_overlaps(interval(a,'2020-01-01T00:00:00Z'),interval('2020-01-01T00:00:00Z','2020-02-01T00:00:00Z'))".parse().unwrap();
    /// assert_eq!(expr.to_ducksql().unwrap(), "(a < CAST('2020-02-01T00:00:00Z' AS TIMESTAMP WITH TIME ZONE) AND CAST('2020-01-01T00:00:00Z' AS TIMESTAMP WITH TIME ZONE) < CAST('2020-01-01T00:00:00Z' AS TIMESTAMP WITH TIME ZONE) AND CAST('2020-01-01T00:00:00Z' AS TIMESTAMP WITH TIME ZONE) < CAST('2020-02-01T00:00:00Z' AS TIMESTAMP WITH TIME ZONE))");
    /// ```
    /// ```
    /// use cql2::Expr;
    /// use cql2::ToDuckSQL;
    ///
    /// let expr: Expr = "t_overlaps(interval(a,b),interval('2020-01-01T00:00:00Z','2020-02-01T00:00:00Z'))".parse().unwrap();
    /// assert_eq!(expr.to_ducksql().unwrap(), "(a < CAST('2020-02-01T00:00:00Z' AS TIMESTAMP WITH TIME ZONE) AND CAST('2020-01-01T00:00:00Z' AS TIMESTAMP WITH TIME ZONE) < b AND b < CAST('2020-02-01T00:00:00Z' AS TIMESTAMP WITH TIME ZONE))");
    /// ```
    fn to_ducksql(&self) -> Result<String, Error> {
        let ast = self.to_sql_ast()?;
        Ok(ast.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::ToDuckSQL;
    use crate::Expr;

    #[test]
    fn test_to_ducksql() {
        let expr: Expr = "foo = 1".parse().unwrap();
        assert_eq!(expr.to_ducksql().unwrap(), "foo = 1");
    }
}
