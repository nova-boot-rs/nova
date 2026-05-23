/// Describes an index for a document collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoSqlIndex {
    pub name: String,
    pub fields: Vec<String>,
    pub unique: bool,
}

impl NoSqlIndex {
    pub fn new(name: impl Into<String>, fields: Vec<String>, unique: bool) -> Self {
        Self {
            name: name.into(),
            fields,
            unique,
        }
    }
}
