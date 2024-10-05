#![no_std]

use core::marker::PhantomData;

pub trait Context<Executor: ?Sized> {}

#[derive(Clone, Copy)]
pub struct ExplicitContext<'a, Executor>(pub &'a Executor);

impl<'a, Executor> From<&'a Executor> for ExplicitContext<'a, Executor> {
    fn from(value: &'a Executor) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy)]
pub struct ImplicitContext<Executor>(PhantomData<*const Executor>);

impl<Executor> Default for ImplicitContext<Executor> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<'a, Executor> Context<Executor> for ExplicitContext<'a, Executor> {}

impl<Executor> Context<Executor> for ImplicitContext<Executor> {}
