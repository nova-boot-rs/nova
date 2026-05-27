use crate::types::GraphQuery;

pub(crate) fn sanitize_symbol(value: &str) -> String {
    let cleaned = value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>();
    if cleaned.is_empty() {
        "Node".to_string()
    } else {
        cleaned
    }
}

pub struct CypherQueryBuilder {
    query: String,
}

impl CypherQueryBuilder {
    pub fn new() -> Self {
        Self {
            query: String::new(),
        }
    }

    pub fn raw(mut self, fragment: impl AsRef<str>) -> Self {
        if !self.query.is_empty() {
            self.query.push(' ');
        }
        self.query.push_str(fragment.as_ref());
        self
    }

    pub fn match_node(mut self, alias: &str, label: &str) -> Self {
        self.query.push_str(&format!("MATCH ({alias}:{label}) "));
        self
    }

    pub fn where_eq(mut self, alias: &str, field: &str, value: &str) -> Self {
        self.query
            .push_str(&format!("WHERE {alias}.{field} = '{value}' "));
        self
    }

    pub fn return_fields(mut self, fields: impl AsRef<str>) -> Self {
        self.query.push_str(&format!("RETURN {}", fields.as_ref()));
        self
    }

    pub fn build(self) -> GraphQuery {
        GraphQuery::Cypher(self.query.trim().to_string())
    }
}

impl Default for CypherQueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GraphQlQueryBuilder {
    root: String,
    fields: Vec<String>,
    args: Vec<(String, String)>,
}

impl GraphQlQueryBuilder {
    pub fn new(root: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            fields: Vec::new(),
            args: Vec::new(),
        }
    }

    pub fn arg(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.args.push((key.into(), value.into()));
        self
    }

    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.fields.push(field.into());
        self
    }

    pub fn build(self) -> GraphQuery {
        let args = if self.args.is_empty() {
            String::new()
        } else {
            let joined = self
                .args
                .into_iter()
                .map(|(k, v)| format!("{k}: \"{v}\""))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({joined})")
        };

        let fields = if self.fields.is_empty() {
            "id".to_string()
        } else {
            self.fields.join(" ")
        };

        GraphQuery::GraphQl(format!(
            "query {{ {}{} {{ {} }} }}",
            self.root, args, fields
        ))
    }
}
