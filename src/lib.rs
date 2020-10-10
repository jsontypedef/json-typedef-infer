mod hint_set;

use chrono::DateTime;
pub use hint_set::HintSet;
use jtd::form::{self, TypeValue};
use jtd::{Form, Schema};
use serde_json::Value;
use std::collections::HashMap;
use std::collections::HashSet;

#[derive(Debug)]
pub struct Hints<'a> {
    enums: HintSet<'a>,
    values: HintSet<'a>,
    discriminator: HintSet<'a>,
}

impl<'a> Hints<'a> {
    pub fn new(enums: HintSet<'a>, values: HintSet<'a>, discriminator: HintSet<'a>) -> Self {
        Hints {
            enums,
            values,
            discriminator,
        }
    }

    fn sub_hints(&self, key: &str) -> Self {
        Self::new(
            self.enums.sub_hints(key),
            self.values.sub_hints(key),
            self.discriminator.sub_hints(key),
        )
    }

    fn is_enum_active(&self) -> bool {
        self.enums.is_active()
    }

    fn is_values_active(&self) -> bool {
        self.values.is_active()
    }

    fn peek_active_discriminator(&self) -> Option<&str> {
        self.discriminator.peek_active()
    }
}

#[derive(Debug)]
pub enum InferredSchema {
    Unknown,
    Any,
    Bool,
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float64,
    String,
    Timestamp,
    Enum(HashSet<String>),
    Array(Box<InferredSchema>),
    Properties {
        required: HashMap<String, InferredSchema>,
        optional: HashMap<String, InferredSchema>,
    },
    Values(Box<InferredSchema>),
    Discriminator {
        discriminator: String,
        mapping: HashMap<String, InferredSchema>,
    },
    Nullable(Box<InferredSchema>),
}

impl InferredSchema {
    pub fn infer(self, value: Value, hints: &Hints) -> Self {
        match (self, value) {
            // Handle all null-related cases first. After these two branches,
            // neither the current inference nor the incoming data will be null.
            //
            // This will cause a deep tree of Nullable when dealing with a long
            // sequence of nulls.
            //
            // If this proves to be a performance concern, we may want to check
            // if the sub-inference is Nullable, and avoid wrapping in that
            // case.
            (sub_infer @ _, Value::Null) => InferredSchema::Nullable(Box::new(sub_infer)),
            (InferredSchema::Nullable(sub_infer), value @ _) => {
                InferredSchema::Nullable(Box::new(sub_infer.infer(value, hints)))
            }

            (InferredSchema::Unknown, Value::Bool(_)) => InferredSchema::Bool,
            (InferredSchema::Unknown, Value::Number(n)) => minimum_number_type(n),
            (InferredSchema::Unknown, Value::String(s)) => {
                if hints.is_enum_active() {
                    let mut values = HashSet::new();
                    values.insert(s);

                    InferredSchema::Enum(values)
                } else if DateTime::parse_from_rfc3339(&s).is_ok() {
                    InferredSchema::Timestamp
                } else {
                    InferredSchema::String
                }
            }
            (InferredSchema::Unknown, Value::Array(vals)) => {
                let mut sub_infer = InferredSchema::Unknown;
                for (i, v) in vals.into_iter().enumerate() {
                    sub_infer = sub_infer.infer(v, &hints.sub_hints(&i.to_string()));
                }

                InferredSchema::Array(Box::new(sub_infer))
            }
            (InferredSchema::Unknown, Value::Object(mut obj)) => {
                if hints.is_values_active() {
                    let mut sub_infer = InferredSchema::Unknown;
                    for (k, v) in obj {
                        sub_infer = sub_infer.infer(v, &hints.sub_hints(&k));
                    }

                    return InferredSchema::Values(Box::new(sub_infer));
                }

                if let Some(discriminator) = hints.peek_active_discriminator() {
                    if let Some(Value::String(mapping_key)) = obj.remove(discriminator) {
                        let infer_rest = InferredSchema::Unknown.infer(Value::Object(obj), hints);

                        let mut mapping = HashMap::new();
                        mapping.insert(mapping_key.to_owned(), infer_rest);

                        return InferredSchema::Discriminator {
                            discriminator: discriminator.to_owned(),
                            mapping,
                        };
                    }
                }

                let mut props = HashMap::new();
                for (k, v) in obj {
                    let sub_infer = InferredSchema::Unknown.infer(v, &hints.sub_hints(&k));
                    props.insert(k, sub_infer);
                }

                InferredSchema::Properties {
                    required: props,
                    optional: HashMap::new(),
                }
            }
            (InferredSchema::Any, _) => InferredSchema::Any,
            (InferredSchema::Bool, Value::Bool(_)) => InferredSchema::Bool,
            (InferredSchema::Bool, _) => InferredSchema::Any,
            (InferredSchema::Uint8, Value::Number(n)) => match minimum_number_type(n) {
                InferredSchema::Uint8 => InferredSchema::Uint8,
                _ => InferredSchema::Float64,
            },
            (InferredSchema::Int8, Value::Number(n)) => match minimum_number_type(n) {
                InferredSchema::Int8 => InferredSchema::Int8,
                _ => InferredSchema::Float64,
            },
            (InferredSchema::Uint16, Value::Number(n)) => match minimum_number_type(n) {
                InferredSchema::Uint8 | InferredSchema::Int8 | InferredSchema::Uint16 => {
                    InferredSchema::Uint16
                }
                _ => InferredSchema::Float64,
            },
            (InferredSchema::Int16, Value::Number(n)) => match minimum_number_type(n) {
                InferredSchema::Uint8 | InferredSchema::Int8 | InferredSchema::Int16 => {
                    InferredSchema::Int16
                }
                _ => InferredSchema::Float64,
            },
            (InferredSchema::Uint32, Value::Number(n)) => match minimum_number_type(n) {
                InferredSchema::Uint8
                | InferredSchema::Int8
                | InferredSchema::Int16
                | InferredSchema::Uint16
                | InferredSchema::Uint32 => InferredSchema::Uint32,
                _ => InferredSchema::Float64,
            },
            (InferredSchema::Int32, Value::Number(n)) => match minimum_number_type(n) {
                InferredSchema::Uint8
                | InferredSchema::Int8
                | InferredSchema::Int16
                | InferredSchema::Uint16
                | InferredSchema::Int32 => InferredSchema::Int32,
                _ => InferredSchema::Float64,
            },
            (InferredSchema::Float64, Value::Number(_)) => InferredSchema::Float64,
            (InferredSchema::Uint8, _)
            | (InferredSchema::Int8, _)
            | (InferredSchema::Uint16, _)
            | (InferredSchema::Int16, _)
            | (InferredSchema::Uint32, _)
            | (InferredSchema::Int32, _)
            | (InferredSchema::Float64, _) => InferredSchema::Any,
            (InferredSchema::Timestamp, Value::String(s)) => {
                if DateTime::parse_from_rfc3339(&s).is_ok() {
                    InferredSchema::Timestamp
                } else {
                    InferredSchema::String
                }
            }
            (InferredSchema::Timestamp, _) => InferredSchema::Any,
            (InferredSchema::String, Value::String(_)) => InferredSchema::String,
            (InferredSchema::String, _) => InferredSchema::Any,
            (InferredSchema::Enum(mut values), Value::String(s)) => {
                values.insert(s);
                InferredSchema::Enum(values)
            }
            (InferredSchema::Enum(_), _) => InferredSchema::Any,
            (InferredSchema::Array(prior), Value::Array(vals)) => {
                let mut sub_infer = *prior;
                for (i, v) in vals.into_iter().enumerate() {
                    sub_infer = sub_infer.infer(v, &hints.sub_hints(&i.to_string()));
                }

                InferredSchema::Array(Box::new(sub_infer))
            }
            (InferredSchema::Array(_), _) => InferredSchema::Any,
            (
                InferredSchema::Properties {
                    mut required,
                    mut optional,
                },
                Value::Object(map),
            ) => {
                let missing_required_keys: Vec<_> = required
                    .keys()
                    .filter(|k| !map.contains_key(k.clone()))
                    .cloned()
                    .collect();

                for k in missing_required_keys {
                    let sub_infer = required.remove(&k).unwrap();
                    optional.insert(k, sub_infer);
                }

                for (k, v) in map {
                    if required.contains_key(&k) {
                        let sub_infer = required.remove(&k).unwrap().infer(v, &hints.sub_hints(&k));
                        required.insert(k, sub_infer);
                    } else if optional.contains_key(&k) {
                        let sub_infer = optional.remove(&k).unwrap().infer(v, &hints.sub_hints(&k));
                        optional.insert(k, sub_infer);
                    } else {
                        let sub_infer = InferredSchema::Unknown.infer(v, &hints.sub_hints(&k));
                        optional.insert(k, sub_infer);
                    }
                }

                InferredSchema::Properties { required, optional }
            }
            (InferredSchema::Properties { .. }, _) => InferredSchema::Any,
            (InferredSchema::Values(prior), Value::Object(map)) => {
                let mut sub_infer = *prior;
                for (k, v) in map {
                    sub_infer = InferredSchema::Unknown.infer(v, &hints.sub_hints(&k));
                }

                return InferredSchema::Values(Box::new(sub_infer));
            }
            (InferredSchema::Values(_), _) => InferredSchema::Any,
            (
                InferredSchema::Discriminator {
                    discriminator,
                    mut mapping,
                },
                Value::Object(mut obj),
            ) => {
                let mapping_key = obj.remove(&discriminator);
                if let Some(Value::String(mapping_key_str)) = mapping_key {
                    if !mapping.contains_key(&mapping_key_str) {
                        mapping.insert(mapping_key_str.clone(), InferredSchema::Unknown);
                    }

                    let sub_infer = mapping
                        .remove(&mapping_key_str)
                        .unwrap()
                        .infer(Value::Object(obj), hints);
                    mapping.insert(mapping_key_str, sub_infer);

                    InferredSchema::Discriminator {
                        discriminator,
                        mapping,
                    }
                } else {
                    InferredSchema::Any
                }
            }
            (InferredSchema::Discriminator { .. }, _) => InferredSchema::Any,
        }
    }

    pub fn into_schema(self) -> Schema {
        let form = match self {
            InferredSchema::Unknown => Form::Empty,
            InferredSchema::Any => Form::Empty,
            InferredSchema::Bool => Form::Type(form::Type {
                nullable: false,
                type_value: TypeValue::Boolean,
            }),
            InferredSchema::Int8 => Form::Type(form::Type {
                nullable: false,
                type_value: TypeValue::Int8,
            }),
            InferredSchema::Uint8 => Form::Type(form::Type {
                nullable: false,
                type_value: TypeValue::Uint8,
            }),
            InferredSchema::Int16 => Form::Type(form::Type {
                nullable: false,
                type_value: TypeValue::Int16,
            }),
            InferredSchema::Uint16 => Form::Type(form::Type {
                nullable: false,
                type_value: TypeValue::Uint16,
            }),
            InferredSchema::Int32 => Form::Type(form::Type {
                nullable: false,
                type_value: TypeValue::Int32,
            }),
            InferredSchema::Uint32 => Form::Type(form::Type {
                nullable: false,
                type_value: TypeValue::Uint32,
            }),
            InferredSchema::Float64 => Form::Type(form::Type {
                nullable: false,
                type_value: TypeValue::Float64,
            }),
            InferredSchema::String => Form::Type(form::Type {
                nullable: false,
                type_value: TypeValue::String,
            }),
            InferredSchema::Timestamp => Form::Type(form::Type {
                nullable: false,
                type_value: TypeValue::Timestamp,
            }),
            InferredSchema::Enum(values) => Form::Enum(form::Enum {
                nullable: false,
                values,
            }),
            InferredSchema::Array(sub_infer) => Form::Elements(form::Elements {
                nullable: false,
                schema: Box::new(sub_infer.into_schema()),
            }),
            InferredSchema::Properties { required, optional } => {
                let has_required = !required.is_empty();

                Form::Properties(form::Properties {
                    nullable: false,
                    required: required
                        .into_iter()
                        .map(|(k, v)| (k, v.into_schema()))
                        .collect(),
                    optional: optional
                        .into_iter()
                        .map(|(k, v)| (k, v.into_schema()))
                        .collect(),
                    has_required,
                    additional: false,
                })
            }
            InferredSchema::Values(sub_infer) => Form::Values(form::Values {
                nullable: false,
                schema: Box::new(sub_infer.into_schema()),
            }),
            InferredSchema::Discriminator {
                discriminator,
                mapping,
            } => Form::Discriminator(form::Discriminator {
                nullable: false,
                discriminator,
                mapping: mapping
                    .into_iter()
                    .map(|(k, v)| (k, v.into_schema()))
                    .collect(),
            }),
            InferredSchema::Nullable(sub_infer) => match sub_infer.into_schema().form {
                Form::Empty => Form::Empty,
                Form::Ref(form::Ref { definition, .. }) => Form::Ref(form::Ref {
                    nullable: true,
                    definition,
                }),
                Form::Type(form::Type { type_value, .. }) => Form::Type(form::Type {
                    nullable: true,
                    type_value,
                }),
                Form::Enum(form::Enum { values, .. }) => Form::Enum(form::Enum {
                    nullable: true,
                    values,
                }),
                Form::Elements(form::Elements { schema, .. }) => Form::Elements(form::Elements {
                    nullable: true,
                    schema,
                }),
                Form::Properties(form::Properties {
                    required,
                    optional,
                    has_required,
                    additional,
                    ..
                }) => Form::Properties(form::Properties {
                    nullable: true,
                    required,
                    optional,
                    has_required,
                    additional,
                }),
                Form::Values(form::Values { schema, .. }) => Form::Values(form::Values {
                    nullable: true,
                    schema,
                }),
                Form::Discriminator(form::Discriminator {
                    discriminator,
                    mapping,
                    ..
                }) => Form::Discriminator(form::Discriminator {
                    nullable: true,
                    discriminator,
                    mapping,
                }),
            },
        };

        Schema {
            metadata: HashMap::new(),
            definitions: HashMap::new(),
            form,
        }
    }
}

fn minimum_number_type(n: serde_json::Number) -> InferredSchema {
    let n = n.as_f64().unwrap();
    if n.fract() != 0.0 {
        return InferredSchema::Float64;
    }

    if n >= 0.0 && n <= 255.0 {
        InferredSchema::Uint8
    } else if n >= -128.0 && n <= 127.0 {
        InferredSchema::Int8
    } else if n >= 0.0 && n <= 65535.0 {
        InferredSchema::Uint16
    } else if n >= -32768.0 && n <= 32767.0 {
        InferredSchema::Int16
    } else if n >= 0.0 && n <= 4294967295.0 {
        InferredSchema::Uint32
    } else if n >= -2147483648.0 && n <= 2147483647.0 {
        InferredSchema::Int32
    } else {
        InferredSchema::Float64
    }
}
