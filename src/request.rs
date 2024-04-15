use tonic::{metadata::MetadataMap, IntoRequest, Request};

pub struct RequestWithToken<'a, T> {
    pub request: T,
    pub token: &'a str,
}

impl<'a, T> IntoRequest<T> for RequestWithToken<'a, T> {
    fn into_request(self) -> Request<T> {
        let mut metadata = MetadataMap::new();
        metadata.insert::<&str>("token", self.token.parse().unwrap());

        Request::from_parts(metadata, tonic::Extensions::default(), self.request)
    }
}
