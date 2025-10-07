use core::future::Future;
use core::marker;
use core::pin::Pin;

pub struct Pending<T> {
    _data: marker::PhantomData<T>,
}

/// A future that never resolves, just exists for the types that don't have a time based transition
pub fn pending<T>() -> Pending<T> {
    Pending {
        _data: marker::PhantomData,
    }
}

impl<T> Future for Pending<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, _cx: &mut core::task::Context<'_>) -> core::task::Poll<T> {
        core::task::Poll::Pending
    }
}
