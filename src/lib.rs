mod client;
mod object;
mod prost_ext;
mod relation;
mod request;
mod space;
mod pb {
    pub(crate) mod models {
        tonic::include_proto!("anytype.model");
    }

    tonic::include_proto!("anytype");
}

pub use client::{AnytypeClient, AuthorizedAnytypeClient, NetworkSync};
pub use relation::{RelationFormat, RelationSpec};
pub use space::Space;
