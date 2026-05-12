use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct FiniteModel {
    pub carrier: Vec<String>,
    pub binary_ops: BTreeMap<String, BTreeMap<(String, String), String>>,
}

impl FiniteModel {
    pub fn eval_binary(&self, op: &str, lhs: &str, rhs: &str) -> Option<&str> {
        self.binary_ops
            .get(op)?
            .get(&(lhs.to_string(), rhs.to_string()))
            .map(String::as_str)
    }
}
