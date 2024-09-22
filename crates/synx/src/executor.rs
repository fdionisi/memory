use std::{future::Future, pin::Pin};

pub trait Executor: Send + Sync {
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send + 'static>>);
}
