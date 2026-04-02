use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    String,
    Date,
    Datetime,
    Bool,
    Number,
    ArrayString,
    Enum(Vec<String>),
    Const(String),
    Link { ref_type: Option<String> },
}

#[derive(Debug, Clone)]
pub struct FieldDefinition {
    pub field_type: FieldType,
    pub required: bool,
    pub default: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TypeDefinition {
    pub name: String,
    pub folder: String,
    pub required: Vec<String>,
    pub fields: HashMap<String, FieldDefinition>,
}
