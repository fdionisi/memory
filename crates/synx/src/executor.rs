use std::{future::Future, pin::Pin, sync::Arc};

pub trait Executor {
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>);
}
