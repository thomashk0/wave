use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub enum Range {
    Bit(u64),
    Range((i64, i64)),
}

#[derive(Clone, Debug, Serialize)]
pub struct Scope {
    pub kind: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct VariableInfo {
    pub id: String,
    pub vtype: String,
    pub width: u32,
    pub name: String,
    pub range: Option<Range>,
    pub scope: Vec<Scope>,
}
