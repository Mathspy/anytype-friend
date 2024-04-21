use std::{collections::HashSet, fmt::Display, hash::Hash};

pub(crate) trait IntoProstValue {
    fn into_prost(self) -> prost_types::Value;
}

impl IntoProstValue for String {
    fn into_prost(self) -> prost_types::Value {
        prost_types::Value {
            kind: Some(prost_types::value::Kind::StringValue(self)),
        }
    }
}

impl IntoProstValue for f64 {
    fn into_prost(self) -> prost_types::Value {
        prost_types::Value {
            kind: Some(prost_types::value::Kind::NumberValue(self)),
        }
    }
}

impl IntoProstValue for Vec<prost_types::Value> {
    fn into_prost(self) -> prost_types::Value {
        prost_types::Value {
            kind: Some(prost_types::value::Kind::ListValue(
                prost_types::ListValue { values: self },
            )),
        }
    }
}

pub(crate) trait TryFromProst {
    type Input;

    fn try_from_prost(input: Self::Input) -> Result<Self, ProstConversionError>
    where
        Self: Sized;
}

impl TryFromProst for String {
    type Input = prost_types::value::Kind;

    fn try_from_prost(kind: Self::Input) -> Result<Self, ProstConversionError> {
        match kind {
            prost_types::value::Kind::StringValue(string) => Ok(string),
            kind => Err(ProstConversionError::IncorrecKind {
                expected: ProstKind::String,
                received: ProstKind::from(&kind),
            }),
        }
    }
}

impl TryFromProst for f64 {
    type Input = prost_types::value::Kind;

    fn try_from_prost(kind: Self::Input) -> Result<Self, ProstConversionError> {
        match kind {
            prost_types::value::Kind::NumberValue(number) => Ok(number),
            kind => Err(ProstConversionError::IncorrecKind {
                expected: ProstKind::Number,
                received: ProstKind::from(&kind),
            }),
        }
    }
}

macro_rules! try_from_prost_iterator {
    ($kind:expr) => {
        match $kind {
            prost_types::value::Kind::ListValue(list) => list
                .values
                .into_iter()
                .map(|value| value.kind)
                .map(|kind| T::try_from_prost(kind.ok_or(ProstConversionError::KindIsEmpty)?))
                .collect(),
            kind => Err(ProstConversionError::IncorrecKind {
                expected: ProstKind::List,
                received: ProstKind::from(&kind),
            }),
        }
    };
}

impl<T> TryFromProst for Vec<T>
where
    T: TryFromProst<Input = prost_types::value::Kind>,
{
    type Input = prost_types::value::Kind;

    fn try_from_prost(kind: Self::Input) -> Result<Self, ProstConversionError> {
        try_from_prost_iterator!(kind)
    }
}

impl<T> TryFromProst for HashSet<T>
where
    T: TryFromProst<Input = prost_types::value::Kind> + Hash + Eq,
{
    type Input = prost_types::value::Kind;

    fn try_from_prost(kind: Self::Input) -> Result<Self, ProstConversionError> {
        try_from_prost_iterator!(kind)
    }
}

#[derive(Debug)]
pub(crate) enum ProstKind {
    Null,
    Number,
    String,
    Bool,
    Struct,
    List,
}

impl From<&prost_types::value::Kind> for ProstKind {
    fn from(value: &prost_types::value::Kind) -> Self {
        use prost_types::value::Kind;

        match value {
            Kind::NullValue(_) => ProstKind::Null,
            Kind::NumberValue(_) => ProstKind::Number,
            Kind::StringValue(_) => ProstKind::String,
            Kind::BoolValue(_) => ProstKind::Bool,
            Kind::StructValue(_) => ProstKind::Struct,
            Kind::ListValue(_) => ProstKind::List,
        }
    }
}

impl Display for ProstKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProstKind::Null => f.write_str("Null"),
            ProstKind::Number => f.write_str("Number"),
            ProstKind::String => f.write_str("String"),
            ProstKind::Bool => f.write_str("Bool"),
            ProstKind::Struct => f.write_str("Struct"),
            ProstKind::List => f.write_str("List"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum ProstConversionError {
    MissingStructField(&'static str),
    KindIsEmpty,
    IncorrecKind {
        expected: ProstKind,
        received: ProstKind,
    },
    InvalidEnumValue(i32),
}

impl Display for ProstConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProstConversionError::MissingStructField(field) => {
                f.write_str("Expected field ")?;
                f.write_str(field)?;
                f.write_str(" but it is missing")
            }
            ProstConversionError::KindIsEmpty => f.write_str("Kind is empty"),
            ProstConversionError::IncorrecKind { expected, received } => {
                f.write_str("Expected prost kind ")?;
                expected.fmt(f)?;
                f.write_str(" but received ")?;
                received.fmt(f)
            }
            ProstConversionError::InvalidEnumValue(value) => {
                f.write_str("Enum value invalid ")?;
                value.fmt(f)
            }
        }
    }
}

impl std::error::Error for ProstConversionError {}
