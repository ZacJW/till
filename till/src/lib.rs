#![no_std]

use core::marker::PhantomData;

pub trait MaybeExplicit<Context: ?Sized> {}

#[derive(Clone, Copy)]
pub struct ExplicitContext<'a, Context>(pub &'a Context);

impl<'a, Context> From<&'a Context> for ExplicitContext<'a, Context> {
    fn from(value: &'a Context) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy)]
pub struct ImplicitContext<Context>(PhantomData<*const Context>);

impl<Context> Default for ImplicitContext<Context> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<'a, Context> MaybeExplicit<Context> for ExplicitContext<'a, Context> {}

impl<Context> MaybeExplicit<Context> for ImplicitContext<Context> {}
