use core::ops::{Deref, DerefMut};

/// `T: Satisfies<MustBeSend>` if, and only if, `T: Send`
/// 
/// For use with the [Satisfies] trait, which enables being generic over the presence of a trait bound
pub struct MustBeSend;

/// `T: Satisfies<MaybeNotSend>` for all `T`
/// 
/// For use with the [Satisfies] trait, which enables being generic over the presence of a trait bound
pub struct MaybeNotSend;

/// Either [MustBeSend] or [MaybeNotSend]
pub trait SendBound: private::Sealed {}

impl SendBound for MustBeSend {}
impl private::Sealed for MustBeSend {}

impl SendBound for MaybeNotSend {}
impl private::Sealed for MaybeNotSend {}

mod private {
    pub trait Sealed {}

    pub trait SealedWith<Bound> {}
}

/// This trait facilitates being generic over the presence of a trait bound
pub trait Satisfies<Bound>: private::SealedWith<Bound> {}

impl<T: ?Sized> Satisfies<MaybeNotSend> for T {}
impl<T: ?Sized> private::SealedWith<MaybeNotSend> for T {}

impl<T: Send + ?Sized> Satisfies<MustBeSend> for T {}
impl<T: Send + ?Sized> private::SealedWith<MustBeSend> for T {}

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
