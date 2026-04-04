use std::collections::HashMap;

/// A single target in a polymorphic link relation.
#[derive(Debug, Clone, PartialEq)]
pub struct PolyTarget {
    /// The entity type this link can point to.
    pub ref_type: String,
    /// The inverse field name on the target entity (only for bidirectional links).
    pub inverse: Option<String>,
}

/// Describes which entity types a link field can target.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkTargets {
    /// `ref: "goal"` — points to exactly one entity type.
    Single {
        ref_type: String,
        inverse: Option<String>,
    },
    /// `ref: [goal, task]` or `ref: { goal: { inverse: ... }, ... }` — polymorphic.
    Poly(Vec<PolyTarget>),
}

/// Full metadata for a link or array[link] field.
///
/// # Cardinality inference rules
/// - `FieldType::Link(def)` — owning side holds one reference; inverse defaults to `array[link]` (many-to-one)
/// - `FieldType::ArrayLink(def)` — owning side holds many references; inverse is `array[link]` (many-to-many)
/// - `FieldType::Link(def)` with `inverse_one: true` — both sides hold one reference (one-to-one)
#[derive(Debug, Clone, PartialEq)]
pub struct LinkDef {
    pub targets: LinkTargets,
    pub bidirectional: bool,
    /// When true, the inverse is also a single link (one-to-one). Default: inverse is array[link].
    pub inverse_one: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    String,
    Date,
    Datetime,
    Bool,
    Number,
    ArrayString,
    Enum(Vec<std::string::String>),
    Const(std::string::String),
    /// A single link to one or more entity types.
    Link(LinkDef),
    /// An array of links to one or more entity types.
    ArrayLink(LinkDef),
}

#[derive(Debug, Clone)]
pub struct FieldDefinition {
    pub field_type: FieldType,
    pub required: bool,
    pub default: Option<std::string::String>,
}

#[derive(Debug, Clone)]
pub struct TypeDefinition {
    pub name: std::string::String,
    pub folder: std::string::String,
    pub required: Vec<std::string::String>,
    pub fields: HashMap<std::string::String, FieldDefinition>,
}
