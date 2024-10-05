use core::ops::{Deref, DerefMut};

pub struct MustBeSend;
pub struct MaybeNotSend;

pub trait SendBound: private::Sealed {}

impl SendBound for MustBeSend {}
impl private::Sealed for MustBeSend {}

impl SendBound for MaybeNotSend {}
impl private::Sealed for MaybeNotSend {}

mod private {
    pub trait Sealed {}

    pub trait SealedWith<Bound> {}
}

// pub trait Holds {}

pub trait Satisfies<Bound>: private::SealedWith<Bound> {}

impl<T: ?Sized> Satisfies<MaybeNotSend> for T {}
impl<T: ?Sized> private::SealedWith<MaybeNotSend> for T {}

impl<T: Send + ?Sized> Satisfies<MustBeSend> for T {}
impl<T: Send + ?Sized> private::SealedWith<MustBeSend> for T {}

// pub struct Assert<T: ?Sized, Bound>(PhantomData<*const (Bound, T)>);

// impl<T: ?Sized> Holds for Assert<T, MaybeNotSend> {}

// impl<T: Send + ?Sized> Holds for Assert<T, MustBeSend> {}

pub trait EagerBlockingHandle {
    type Return;
}

pub trait EagerBlockingImplicit<SendBound: self::SendBound> {
    type Handle<T>: EagerBlockingHandle<Return = T>;

    fn spawn_eager_blocking_implicit<
        T: Satisfies<SendBound> + 'static,
        F: FnOnce() -> T + Satisfies<SendBound> + 'static,
    >(
        f: F,
    ) -> Self::Handle<T>;
}

#[repr(transparent)]
pub struct Sendable<T: Satisfies<MustBeSend>> {
    inner: T,
}

impl<T: Satisfies<MustBeSend>> Sendable<T> {
    pub fn new(value: T) -> Self {
        Self { inner: value }
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T: Satisfies<MustBeSend>> Deref for Sendable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Satisfies<MustBeSend>> DerefMut for Sendable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

unsafe impl<T: Satisfies<MustBeSend>> Send for Sendable<T> {}
