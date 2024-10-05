#![no_std]

pub mod maybe_send;

use core::{
    future::Future,
    pin::{pin, Pin},
};

use maybe_send::Satisfies;

// use maybe_send::{Assert, Holds};

/// Executor support for spawning blocking functions on a thread that won't block async tasks
/// from making progress.
pub trait Blocking<SendBound: maybe_send::SendBound> {
    type Node<T: Satisfies<SendBound> + 'static>: BlockingNode<Output = Result<T, Self::Error>>;
    type Error;

    fn spawn_blocking<
        T: Satisfies<SendBound> + 'static,
        F: FnOnce() -> T + Satisfies<SendBound> + 'static,
    >(
        &self,
        node: Pin<&mut Self::Node<T>>,
        f: F,
    );
}

/// Executor support for spawning blocking functions on a thread that won't block async tasks
/// from making progress.
///
/// Unlike [Blocking], this trait uses an implicit context mechanism to register the blocking
/// function with the executor.
pub trait BlockingImplicit<SendBound: maybe_send::SendBound> {
    type Node<T: Satisfies<SendBound> + 'static>: BlockingNode<Output = Result<T, Self::Error>>;
    type Error;

    fn spawn_blocking_implicit<
        T: Satisfies<SendBound> + 'static,
        F: FnOnce() -> T + Satisfies<SendBound> + 'static,
    >(
        node: Pin<&mut Self::Node<T>>,
        f: F,
    );
}

pub trait BlockingNode: Future {
    fn new_empty() -> Self;
}

pub async fn spawn_blocking<
    SendBound: maybe_send::SendBound,
    T: Satisfies<SendBound> + 'static,
    F: FnOnce() -> T + Satisfies<SendBound> + 'static,
    B: Blocking<SendBound>,
>(
    blocking: &B,
    f: F,
) -> Result<T, <B as Blocking<SendBound>>::Error> {
    let mut node = pin!(B::Node::<T>::new_empty());
    blocking.spawn_blocking(node.as_mut(), f);
    node.await
}

pub async fn spawn_blocking_implicit<
    SendBound: maybe_send::SendBound,
    T: Satisfies<SendBound> + 'static,
    F: FnOnce() -> T + Satisfies<SendBound> + 'static,
    B: BlockingImplicit<SendBound>,
>(
    f: F,
) -> Result<T, <B as BlockingImplicit<SendBound>>::Error> {
    let mut node = pin!(B::Node::<T>::new_empty());
    B::spawn_blocking_implicit(node.as_mut(), f);
    node.await
}

/// Executor support for spawning blocking functions on a thread that won't block async tasks
/// from making progress.
///
/// Unlike [Blocking], this trait allow functions to be spawned in an eager way (i.e. before the first await of the join handle)
pub trait EagerBlocking<SendBound: maybe_send::SendBound> {
    type Handle<T>: EagerBlockingHandle<Return = T>;

    fn spawn_eager_blocking<
        T: Satisfies<SendBound> + 'static,
        F: FnOnce() -> T + Satisfies<SendBound> + 'static,
    >(
        &self,
        f: F,
    ) -> Self::Handle<T>;
}

pub trait EagerBlockingHandle {
    type Return;
}

/// Executor support for spawning blocking functions on a thread that won't block async tasks
/// from making progress.
///
/// Unlike [EagerBlocking], this trait uses an implicit context mechanism to register the blocking
/// function with the executor.
pub trait EagerBlockingImplicit<SendBound: maybe_send::SendBound> {
    type Handle<T>: EagerBlockingHandle<Return = T>;

    fn spawn_eager_blocking_implicit<
        T: Satisfies<SendBound> + 'static,
        F: FnOnce() -> T + Satisfies<SendBound> + 'static,
    >(
        f: F,
    ) -> Self::Handle<T>;
}
