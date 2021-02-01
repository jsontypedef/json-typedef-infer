use crate::hints::Hints;
use crate::inferred_number::InferredNumber;
use chrono::DateTime;
use jtd::{Schema, Type};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub enum InferredSchema {
    Unknown,
    Any,
    Boolean,
    Number(InferredNumber),
    String,
    Timestamp,
    Enum(BTreeSet<String>),
    Array(Box<InferredSchema>),
    Properties {
        required: BTreeMap<String, InferredSchema>,
        optional: BTreeMap<String, InferredSchema>,
    },
    Values(Box<InferredSchema>),
    Discriminator {
        discriminator: String,
        mapping: BTreeMap<String, InferredSchema>,
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

            // Handle all cases related to when we don't have a prior on what
            // the data should be.
            //
            // These cases are where we allow hints to tell us to use a
            // particular form.
            (InferredSchema::Unknown, Value::Bool(_)) => InferredSchema::Boolean,
            (InferredSchema::Unknown, Value::Number(n)) => {
                InferredSchema::Number(InferredNumber::new().infer(n.as_f64().unwrap()))
            }
            (InferredSchema::Unknown, Value::String(s)) => {
                if hints.is_enum_active() {
                    let mut values = BTreeSet::new();
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

                        let mut mapping = BTreeMap::new();
                        mapping.insert(mapping_key.to_owned(), infer_rest);

                        return InferredSchema::Discriminator {
                            discriminator: discriminator.to_owned(),
                            mapping,
                        };
                    }
                }

                let mut props = BTreeMap::new();
                for (k, v) in obj {
                    let sub_infer = InferredSchema::Unknown.infer(v, &hints.sub_hints(&k));
                    props.insert(k, sub_infer);
                }

                InferredSchema::Properties {
                    required: props,
                    optional: BTreeMap::new(),
                }
            }

            // Handle updating an inferred "any". Sort of a trivial case; once
            // we've inferred something can be "anything", we'll never narrow it
            // down thereafter.
            (InferredSchema::Any, _) => InferredSchema::Any,

            // Handle updating an inferred boolean primitive.
            (InferredSchema::Boolean, Value::Bool(_)) => InferredSchema::Boolean,
            (InferredSchema::Boolean, _) => InferredSchema::Any,

            // Handle updating an inferred number primitive.
            (InferredSchema::Number(inferred_number), Value::Number(n)) => {
                InferredSchema::Number(inferred_number.infer(n.as_f64().unwrap()))
            }
            (InferredSchema::Number(_), _) => InferredSchema::Any,

            // Handle updating an inferred timestamp primitive.
            (InferredSchema::Timestamp, Value::String(s)) => {
                if DateTime::parse_from_rfc3339(&s).is_ok() {
                    InferredSchema::Timestamp
                } else {
                    InferredSchema::String
                }
            }
            (InferredSchema::Timestamp, _) => InferredSchema::Any,

            // Handle updating an inferred string primitive.
            (InferredSchema::String, Value::String(_)) => InferredSchema::String,
            (InferredSchema::String, _) => InferredSchema::Any,

            // Handle updating an inferred enum.
            (InferredSchema::Enum(mut values), Value::String(s)) => {
                values.insert(s);
                InferredSchema::Enum(values)
            }
            (InferredSchema::Enum(_), _) => InferredSchema::Any,

            // Handle updating an inferred array.
            (InferredSchema::Array(prior), Value::Array(vals)) => {
                let mut sub_infer = *prior;
                for (i, v) in vals.into_iter().enumerate() {
                    sub_infer = sub_infer.infer(v, &hints.sub_hints(&i.to_string()));
                }

                InferredSchema::Array(Box::new(sub_infer))
            }
            (InferredSchema::Array(_), _) => InferredSchema::Any,

            // Handle updating an inferred properties form.
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

            // Handle updating an inferred values form.
            (InferredSchema::Values(prior), Value::Object(map)) => {
                let mut sub_infer = *prior;
                for (k, v) in map {
                    sub_infer = InferredSchema::Unknown.infer(v, &hints.sub_hints(&k));
                }

                return InferredSchema::Values(Box::new(sub_infer));
            }
            (InferredSchema::Values(_), _) => InferredSchema::Any,

            // Handle updating an inferred discriminator form.
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

    pub fn into_schema(self, hints: &Hints) -> Schema {
        match self {
            InferredSchema::Unknown | InferredSchema::Any => Schema::Empty {
                definitions: Default::default(),
                metadata: Default::default(),
            },
            InferredSchema::Boolean => Schema::Type {
                definitions: Default::default(),
                metadata: Default::default(),
                nullable: false,
                type_: Type::Boolean,
            },
            InferredSchema::Number(inferred_number) => Schema::Type {
                definitions: Default::default(),
                metadata: Default::default(),
                nullable: false,
                type_: inferred_number.into_type(hints.default_num_type()),
            },
            InferredSchema::String => Schema::Type {
                definitions: Default::default(),
                metadata: Default::default(),
                nullable: false,
                type_: Type::String,
            },
            InferredSchema::Timestamp => Schema::Type {
                definitions: Default::default(),
                metadata: Default::default(),
                nullable: false,
                type_: Type::Timestamp,
            },
            InferredSchema::Enum(values) => Schema::Enum {
                definitions: Default::default(),
                metadata: Default::default(),
                nullable: false,
                enum_: values,
            },
            InferredSchema::Array(sub_infer) => Schema::Elements {
                definitions: Default::default(),
                metadata: Default::default(),
                nullable: false,
                elements: Box::new(sub_infer.into_schema(hints)),
            },
            InferredSchema::Properties { required, optional } => {
                let properties_is_present = !required.is_empty();

                Schema::Properties {
                    definitions: Default::default(),
                    metadata: Default::default(),
                    nullable: false,
                    properties: required
                        .into_iter()
                        .map(|(k, v)| (k, v.into_schema(hints)))
                        .collect(),
                    optional_properties: optional
                        .into_iter()
                        .map(|(k, v)| (k, v.into_schema(hints)))
                        .collect(),
                    properties_is_present,
                    additional_properties: false,
                }
            }
            InferredSchema::Values(sub_infer) => Schema::Values {
                definitions: Default::default(),
                metadata: Default::default(),
                nullable: false,
                values: Box::new(sub_infer.into_schema(hints)),
            },
            InferredSchema::Discriminator {
                discriminator,
                mapping,
            } => Schema::Discriminator {
                definitions: Default::default(),
                metadata: Default::default(),
                nullable: false,
                discriminator,
                mapping: mapping
                    .into_iter()
                    .map(|(k, v)| (k, v.into_schema(hints)))
                    .collect(),
            },
            InferredSchema::Nullable(sub_infer) => match sub_infer.into_schema(hints) {
                Schema::Ref { .. } => unreachable!("ref form inferred"),

                s @ Schema::Empty { .. } => s,
                Schema::Type {
                    definitions,
                    metadata,
                    type_,
                    ..
                } => Schema::Type {
                    definitions,
                    metadata,
                    nullable: true,
                    type_,
                },
                Schema::Enum {
                    definitions,
                    metadata,
                    enum_,
                    ..
                } => Schema::Enum {
                    definitions,
                    metadata,
                    nullable: true,
                    enum_,
                },
                Schema::Elements {
                    definitions,
                    metadata,
                    elements,
                    ..
                } => Schema::Elements {
                    definitions,
                    metadata,
                    nullable: true,
                    elements,
                },
                Schema::Properties {
                    definitions,
                    metadata,
                    properties,
                    optional_properties,
                    properties_is_present,
                    additional_properties,
                    ..
                } => Schema::Properties {
                    definitions,
                    metadata,
                    nullable: true,
                    properties,
                    optional_properties,
                    properties_is_present,
                    additional_properties,
                },
                Schema::Values {
                    definitions,
                    metadata,
                    values,
                    ..
                } => Schema::Values {
                    definitions,
                    metadata,
                    nullable: true,
                    values,
                },
                Schema::Discriminator {
                    definitions,
                    metadata,
                    discriminator,
                    mapping,
                    ..
                } => Schema::Discriminator {
                    definitions,
                    metadata,
                    nullable: true,
                    discriminator,
                    mapping,
                },
            },
        }
    }
}
