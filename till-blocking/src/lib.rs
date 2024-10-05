#![no_std]

pub mod maybe_send;

use core::{
    future::Future,
    pin::{pin, Pin},
};

use till::{ExplicitContext, ImplicitContext};
use maybe_send::Satisfies;

/// Executor support for spawning blocking functions on a thread that won't block async tasks
/// from making progress.
pub trait Blocking<SendBound: maybe_send::SendBound, Context: till::MaybeExplicit<Self>> {
    type Node<T: Satisfies<SendBound> + 'static>: BlockingNode<Output = Result<T, Self::Error>>;
    type Error;

    fn spawn_blocking<
        T: Satisfies<SendBound> + 'static,
        F: FnOnce() -> T + Satisfies<SendBound> + 'static,
    >(
        context: Context,
        node: Pin<&mut Self::Node<T>>,
        f: F,
    );
}

pub trait BlockingNode: Future {
    fn new_empty() -> Self;
}

pub async fn spawn_blocking_explicit<
    'a,
    SendBound: maybe_send::SendBound,
    T: Satisfies<SendBound> + 'static,
    F: FnOnce() -> T + Satisfies<SendBound> + 'static,
    B: Blocking<SendBound, ExplicitContext<'a, B>>,
>(
    context: &'a B,
    f: F,
) -> Result<T, <B as Blocking<SendBound, ExplicitContext<'a, B>>>::Error> {
    let mut node = pin!(B::Node::<T>::new_empty());
    B::spawn_blocking(context.into(), node.as_mut(), f);
    node.await
}

pub async fn spawn_blocking_implicit<
    SendBound: maybe_send::SendBound,
    T: Satisfies<SendBound> + 'static,
    F: FnOnce() -> T + Satisfies<SendBound> + 'static,
    B: Blocking<SendBound, ImplicitContext<B>>,
>(
    f: F,
) -> Result<T, <B as Blocking<SendBound, ImplicitContext<B>>>::Error> {
    let mut node = pin!(B::Node::<T>::new_empty());
    B::spawn_blocking(Default::default(), node.as_mut(), f);
    node.await
}

/// Executor support for spawning blocking functions on a thread that won't block async tasks
/// from making progress.
///
/// Unlike [Blocking], this trait allow functions to be spawned in an eager way (i.e. before the first await of the join handle)
pub trait EagerBlocking<
    SendBound: maybe_send::SendBound,
    Context: till::MaybeExplicit<Self>,
>
{
    type Handle<T>: EagerBlockingHandle<Return = T>;

    fn spawn_eager_blocking<
        T: Satisfies<SendBound> + 'static,
        F: FnOnce() -> T + Satisfies<SendBound> + 'static,
    >(
        context: Context,
        f: F,
    ) -> Self::Handle<T>;
}

pub trait EagerBlockingHandle {
    type Return;
}
