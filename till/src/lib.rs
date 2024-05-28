#![no_std]
#![feature(waker_getters)]

pub mod impls;

// static CURRENT_EXECUTOR: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());

struct BoundedEventSourceId(usize);

use core::{
    cell::RefCell, future::Future, hint::unreachable_unchecked, pin::Pin, sync::atomic::AtomicPtr,
    usize,
};

use futures::future::FusedFuture;
use impls::no_heap::{SingleThreadMarshall, StreamingIterator};

pub const fn raw_waker_v_table<MarshallType: Marshall>() -> core::task::RawWakerVTable {
    unsafe fn clone<Marshall: self::Marshall>(marshall: *const ()) -> core::task::RawWaker {
        let waker = Marshall::waker(&*(marshall as *const Marshall));
        let raw = waker.as_raw();
        let raw = core::task::RawWaker::new(raw.data(), raw.vtable());
        core::mem::forget(waker);
        raw
    }
    unsafe fn wake<Marshall: self::Marshall>(marshall: *const ()) {
        // Nothing is owned or needs to be cleaned up so this is just wake_by_ref
        wake_by_ref::<Marshall>(marshall)
    }
    unsafe fn wake_by_ref<Marshall: self::Marshall>(marshall: *const ()) {
        let marshall = &*(marshall as *const Marshall);
        marshall.wake();
    }
    unsafe fn drop<Marshall: self::Marshall>(_marshall: *const ()) {
        // The marshall has static lifetime so no cleanup required
    }
    core::task::RawWakerVTable::new(
        clone::<MarshallType>,
        wake::<MarshallType>,
        wake_by_ref::<MarshallType>,
        drop::<MarshallType>,
    )
}

// fn raw_waker<Marshall: ExecutorMarshall>(marshall: &'static Marshall) -> core::task::RawWaker {
//     core::task::RawWaker::new(
//         marshall as *const Marshall as *const (),
//         Marshall::v_table(),
//     )
// }

pub trait Marshall: Sync + 'static {
    fn wake(&'static self);
    fn waker(&'static self) -> core::task::Waker;
}

pub struct Executor<'a, Tasks: TaskManager, Pool: EventSourcePool> {
    tasks: &'a mut Tasks,
    pool: &'a mut Pool,
}

pub enum WakeStatus {
    Woken,
    Asleep,
}

pub trait TaskManager {
    type Marshall: Marshall;
    type TaskIterator<'a>: Iterator<
        Item = (
            Pin<&'a mut dyn FusedFutureWithWakeStatus<Output = ()>>,
            &'static Self::Marshall,
        ),
    >
    where
        Self: 'a;
    fn get_task(
        &mut self,
        i: usize,
    ) -> Option<(
        Pin<&mut dyn FusedFutureWithWakeStatus<Output = ()>>,
        &'static Self::Marshall,
    )>;
    fn sleep_task(&mut self, i: usize);
    fn sleep_all(&mut self);
    fn tasks<'a>(&'a mut self) -> Self::TaskIterator<'a>;
}

pub trait EventSourcePool: Context {
    type SourceIterator<'a>: Iterator<Item = &'a mut Source>
    where
        Self: 'a;

    fn sources<'a>(&'a mut self) -> Self::SourceIterator<'a>;
}

pub trait Context {
    type Id;
    unsafe fn register_source(&self, source: *mut dyn EventSource) -> Option<Self::Id>;
    fn unregister_source(&self, id: Self::Id);
}

struct BoundedEventSourcePool<const N: usize> {
    slots: RefCell<[Option<*mut dyn EventSource>; N]>,
}

impl<const N: usize> Context for BoundedEventSourcePool<N> {
    type Id = BoundedEventSourceId;

    unsafe fn register_source(&self, source: *mut dyn EventSource) -> Option<Self::Id> {
        let mut slots = self.slots.borrow_mut();
        if let Some((i, empty_slot)) = slots
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.is_none())
        {
            *empty_slot = Some(source);
            Some(BoundedEventSourceId(i))
        } else {
            None
        }
    }

    fn unregister_source(&self, id: Self::Id) {
        self.slots.borrow_mut()[id.0] = None;
    }
}

pub trait EventSource {
    fn setup(&mut self);

    fn check(&mut self);

    fn cleanup(self);
}

pub trait FusedFutureWithWakeStatus: FusedFuture {
    fn status(&self) -> WakeStatus;
    fn set_status(self: Pin<&mut Self>, status: WakeStatus);
}

pub trait FusedFutureExt {
    fn with_wake_status_st(self) -> SingleThreadWithWakeStatus<Self>
    where
        Self: Sized;
}

#[pin_project::pin_project]
pub struct SingleThreadWithWakeStatus<F> {
    #[pin]
    f: F,
    status: core::cell::Cell<bool>,
}

impl<F> SingleThreadWithWakeStatus<F> {
    pub unsafe fn register(self: Pin<&mut Self>, marshall: &'static SingleThreadMarshall) {
        let projection = self.project();
        marshall.register(&projection.status);
    }
}

impl<F: Future> Future for SingleThreadWithWakeStatus<F> {
    type Output = F::Output;

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let projection = self.project();
        projection.f.poll(cx)
    }
}

impl<F: FusedFuture> FusedFuture for SingleThreadWithWakeStatus<F> {
    fn is_terminated(&self) -> bool {
        self.f.is_terminated()
    }
}

impl<F: FusedFuture> FusedFutureWithWakeStatus for SingleThreadWithWakeStatus<F> {
    fn status(&self) -> WakeStatus {
        if self.status.get() {
            WakeStatus::Woken
        } else {
            WakeStatus::Asleep
        }
    }

    fn set_status(self: Pin<&mut Self>, status: WakeStatus) {
        match status {
            WakeStatus::Woken => self.status.set(true),
            WakeStatus::Asleep => self.status.set(false),
        }
    }
}

impl<F: futures::future::FusedFuture> FusedFutureExt for F {
    fn with_wake_status_st(self) -> SingleThreadWithWakeStatus<Self> {
        SingleThreadWithWakeStatus {
            f: self,
            status: core::cell::Cell::new(true),
        }
    }
}

pub enum Source {
    Poll {},
}

impl<'a, Tasks: TaskManager, Pool: EventSourcePool> Executor<'a, Tasks, Pool> {
    pub fn new(tasks: &'a mut Tasks, pool: &'a mut Pool) -> Self {
        Executor { tasks, pool }
    }

    pub fn run_to_completion(self) {
        while self.tasks.tasks().any(|(task, _)| !task.is_terminated()) {
            for (i, (mut task, marshall)) in
                self.tasks.tasks().enumerate().filter(|(_, (task, _))| {
                    !task.is_terminated() && matches!(task.status(), WakeStatus::Woken)
                })
            {
                let waker = marshall.waker();
                let mut context: core::task::Context<'_> = core::task::Context::from_waker(&waker);
                task.as_mut().set_status(WakeStatus::Asleep);
                let _ = task.poll(&mut context);
            }
            // TODO - Process event source pool
        }
    }
}
