use crate::prost_ext::{ProstConversionError, TryFromProst};

#[derive(Debug, Clone)]
pub struct UniqueKey(pub(crate) String);

impl TryFromProst for UniqueKey {
    type Input = prost_types::value::Kind;

    fn try_from_prost(kind: Self::Input) -> Result<Self, ProstConversionError> {
        String::try_from_prost(kind).map(UniqueKey)
    }
}
