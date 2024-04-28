mod client;
mod object;
mod object_type;
mod prost_ext;
mod relation;
mod request;
mod space;
mod unique_key;
mod pb {
    pub(crate) mod models {
        tonic::include_proto!("anytype.model");
    }

    tonic::include_proto!("anytype");
}

pub use client::{AnytypeClient, AuthorizedAnytypeClient, NetworkSync};
pub use object::ObjectDescription;
pub use object_type::ObjectTypeSpec;
pub use relation::{Relation, RelationFormat, RelationSpec, RelationValue};
pub use space::Space;
