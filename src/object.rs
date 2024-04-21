use std::fmt::Display;

use crate::prost_ext::{ProstConversionError, TryFromProst};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ObjectId(String);

impl Display for ObjectId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFromProst for ObjectId {
    type Input = prost_types::value::Kind;

    fn try_from_prost(kind: Self::Input) -> Result<Self, ProstConversionError> {
        String::try_from_prost(kind).map(ObjectId)
    }
}
