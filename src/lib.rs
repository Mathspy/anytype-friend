mod client;
mod prost_ext;
mod request;
mod space;
mod pb {
    pub(crate) mod models {
        tonic::include_proto!("anytype.model");
    }

    tonic::include_proto!("anytype");
}

pub use client::{AnytypeClient, AuthorizedAnytypeClient, NetworkSync};
pub use space::Space;
