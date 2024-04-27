use std::{collections::BTreeSet, fmt::Display, hash::Hash};

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

impl TryFromProst for bool {
    type Input = prost_types::value::Kind;

    fn try_from_prost(kind: Self::Input) -> Result<Self, ProstConversionError> {
        match kind {
            prost_types::value::Kind::BoolValue(boolean) => Ok(boolean),
            kind => Err(ProstConversionError::IncorrecKind {
                expected: ProstKind::Bool,
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

impl<T> TryFromProst for BTreeSet<T>
where
    T: TryFromProst<Input = prost_types::value::Kind> + Hash + Ord,
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

pub(crate) struct ProstStruct {
    inner: prost_types::Struct,
}

impl From<prost_types::Struct> for ProstStruct {
    fn from(value: prost_types::Struct) -> Self {
        Self { inner: value }
    }
}

impl ProstStruct {
    fn take_inner(
        &mut self,
        field: &'static str,
    ) -> Result<prost_types::value::Kind, ProstConversionError> {
        self.inner
            .fields
            .remove(field)
            .ok_or(ProstConversionError::MissingStructField(field))?
            .kind
            .ok_or(ProstConversionError::KindIsEmpty)
    }

    pub fn take<T>(&mut self, field: &'static str) -> Result<T, ProstConversionError>
    where
        T: TryFromProst<Input = prost_types::value::Kind>,
    {
        T::try_from_prost(self.take_inner(field)?)
    }

    pub fn take_optional<T>(
        &mut self,
        field: &'static str,
    ) -> Result<Option<T>, ProstConversionError>
    where
        T: TryFromProst<Input = prost_types::value::Kind>,
    {
        match self.take_inner(field) {
            Ok(kind) => T::try_from_prost(kind).map(Some),
            Err(ProstConversionError::MissingStructField(_)) => Ok(None),
            Err(error) => Err(error),
        }
    }

    pub fn take_enum<E: TryFrom<i32, Error = prost::DecodeError>>(
        &mut self,
        field: &'static str,
    ) -> Result<E, ProstConversionError> {
        let number = self.take::<f64>(field)? as i32;

        E::try_from(number).map_err(|_| ProstConversionError::InvalidEnumValue(number))
    }
}
