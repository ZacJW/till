#![no_std]
use core::{future::poll_fn, pin::{pin, Pin}, task::{Context, Poll}};

/// Executor support for spawning blocking functions on a thread that won't block async tasks
/// from making progress.
pub trait Blocking {
    type Node<T: Send + 'static>: BlockingNode<Return = T>;

    fn spawn_blocking<T: Send + 'static>(&self, node: Pin<&mut Self::Node<T>>, f: impl FnOnce() -> T + Send + 'static);
}

/// Executor support for spawning blocking functions on a thread that won't block async tasks
/// from making progress.
/// 
/// Unlike [Blocking], this trait uses an implicit context mechanism to register the blocking
/// function with the executor.
pub trait BlockingImplicit {
    type Node<T: Send + 'static>: BlockingNode<Return = T>;

    fn spawn_blocking<T: Send + 'static>(node: Pin<&mut Self::Node<T>>, f: impl FnOnce() -> T + Send + 'static);
}

pub trait BlockingNode {
    type Return: Send + 'static;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Return>;

    fn new_empty() -> Self;
}

pub async fn spawn_blocking<T: Send + 'static, F: FnOnce() -> T + Send + 'static, B: Blocking>(blocking: &B, f: F) -> T {
    let mut node = pin!(B::Node::<T>::new_empty());
    blocking.spawn_blocking(node.as_mut(), f);
    poll_fn(move |cx| node.as_mut().poll(cx)).await
}

pub async fn spawn_blocking_implicit<T: Send + 'static, F: FnOnce() -> T + Send + 'static, B: BlockingImplicit>(f: F) -> T {
    let mut node = pin!(B::Node::<T>::new_empty());
    B::spawn_blocking(node.as_mut(), f);
    poll_fn(move |cx| node.as_mut().poll(cx)).await
}
