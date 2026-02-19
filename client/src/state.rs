use std::{ops::Deref, sync::Arc};

#[derive(Clone)]
pub struct AppState {
    pub reqwest_client: reqwest::Client,
    pub nl_handle: rtnetlink::Handle,
}

#[derive(Clone)]
pub struct Data<T>(Arc<T>);

impl<T> Deref for Data<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Data<T> {
    pub fn new(inner: T) -> Self {
        Self(Arc::new(inner))
    }
}
