use core::{cell::Cell, pin::Pin, task::RawWakerVTable};

use crate::{
    Context, EventSourcePool, ExecutorMarshall, FusedFutureWithWakeStatus, Source, TaskManager,
};

struct AtomicPtrMarshall {
    lock: core::sync::atomic::AtomicBool,
    ptr: core::sync::atomic::AtomicPtr<core::sync::atomic::AtomicBool>,
}

static ATOMIC_PTR_MARSHALL_VTABLE: RawWakerVTable = crate::raw_waker_v_table::<AtomicPtrMarshall>();

impl ExecutorMarshall for AtomicPtrMarshall {
    fn wake(&'static self) {
        todo!()
    }

    fn waker(&'static self) -> core::task::Waker {
        unsafe {
            core::task::Waker::from_raw(core::task::RawWaker::new(
                self as *const Self as *const (),
                &ATOMIC_PTR_MARSHALL_VTABLE,
            ))
        }
    }
}

static SINGLE_THREAD_MARSHALL_VTABLE: RawWakerVTable =
    crate::raw_waker_v_table::<SingleThreadMarshall>();

/// An executor marshall that is only safe to be used from a single thread.
///
/// Its implementation of [Sync] is only so that you can have static values.
pub struct SingleThreadMarshall {
    ptr: Cell<*const Cell<bool>>,
}

// SAFETY: creating values of this type is unsafe and requires upholding a
// contract that ensures no UB can occur due to what would otherwise be an
// unsound impl of Sync for this type.
unsafe impl Sync for SingleThreadMarshall {}

impl SingleThreadMarshall {
    /// You must not share references to this value between threads. All method calls
    /// on this value must come from the same thread for its entire lifetime.
    ///
    /// If this was called outside of a static initialiser, the value can only be used
    /// from the thread that created it.
    ///
    /// Its implementation of [Sync] is only so that you can have static values.
    pub const unsafe fn new() -> Self {
        Self {
            ptr: Cell::new(core::ptr::null()),
        }
    }

    pub unsafe fn register(&self, b: &Cell<bool>) {
        self.ptr.set(b);
    }

    pub unsafe fn unregister(&self) {
        self.ptr.set(core::ptr::null());
    }
}

impl ExecutorMarshall for SingleThreadMarshall {
    fn wake(&'static self) {
        let ptr = self.ptr.get();
        if ptr.is_null() {
            return;
        }
        unsafe {
            (&*ptr).set(true);
        }
    }

    fn waker(&'static self) -> core::task::Waker {
        unsafe {
            core::task::Waker::from_raw(core::task::RawWaker::new(
                self as *const Self as *const (),
                &SINGLE_THREAD_MARSHALL_VTABLE,
            ))
        }
    }
}

pub struct ArrayTaskManager<'a, const N: usize, Marshall: ExecutorMarshall> {
    pub tasks: [(
        Pin<&'a mut dyn FusedFutureWithWakeStatus<Output = ()>>,
        &'static Marshall,
    ); N],
}

pub trait StreamingIterator {
    type Item<'n>
    where
        Self: 'n;

    fn next<'n>(&'n mut self) -> Option<Self::Item<'n>>;
}

pub struct ArrayTaskManagerIter<'a, 'b: 'a, const N: usize, Marshall: ExecutorMarshall> {
    array: &'a mut [(
        Pin<&'b mut dyn FusedFutureWithWakeStatus<Output = ()>>,
        &'static Marshall,
    ); N],
    i: usize,
}

impl<'a, 'b: 'a, const N: usize, Marshall: ExecutorMarshall> StreamingIterator for ArrayTaskManagerIter<'a, 'b, N, Marshall> {
    type Item<'n> = (Pin<&'n mut dyn FusedFutureWithWakeStatus<Output = ()>>, &'static Marshall)
    where
        Self: 'n;

    fn next<'n>(&'n mut self) -> Option<Self::Item<'n>> {
        self.array.get_mut(self.i).map(move |(task, marshall)| {
            (unsafe {core::mem::transmute(task.as_mut())}, *marshall)
        })
    }
}


impl<'a, 'b, const N: usize, Marshall: ExecutorMarshall> Iterator
    for ArrayTaskManagerIter<'a, 'b, N, Marshall>
{
    type Item = (
        Pin<&'a mut dyn FusedFutureWithWakeStatus<Output = ()>>,
        &'static Marshall,
    );

    fn next(&mut self) -> Option<Self::Item> {
        if self.i < N {
            let item = &mut (*self.array)[self.i];
            let item = (item.0.as_mut(), item.1);
            self.i += 1;
            // TODO - Check if this lifetime extension is sound
            unsafe { core::mem::transmute(Some(item)) }
        } else {
            None
        }
    }
}

impl<'a, const N: usize, Marshall: ExecutorMarshall> TaskManager
    for ArrayTaskManager<'a, N, Marshall>
{
    type Marshall = Marshall;

    type TaskIterator<'b> = ArrayTaskManagerIter<'b, 'a, N, Marshall>
    where
        Self: 'b;

    fn get_task(
        &mut self,
        i: usize,
    ) -> Option<(
        Pin<&mut dyn FusedFutureWithWakeStatus<Output = ()>>,
        &'static Self::Marshall,
    )> {
        self.tasks
            .get_mut(i)
            .map(|(task, marshall)| (unsafe { core::mem::transmute(task.as_mut()) }, *marshall))
    }

    fn sleep_task(&mut self, i: usize) {
        self.tasks[i]
            .0
            .as_mut()
            .set_status(crate::WakeStatus::Asleep);
    }

    fn sleep_all(&mut self) {
        for task in &mut self.tasks {
            task.0.as_mut().set_status(crate::WakeStatus::Asleep);
        }
    }

    fn tasks<'b>(&'b mut self) -> Self::TaskIterator<'b> {
        ArrayTaskManagerIter {
            array: &mut self.tasks,
            i: 0,
        }
    }
}

pub struct DummyPool;

pub enum DummyId {}

impl Context for DummyPool {
    type Id = DummyId;

    unsafe fn register_source(&self, source: *mut dyn crate::EventSource) -> Option<Self::Id> {
        None
    }

    fn unregister_source(&self, id: Self::Id) {}
}

impl EventSourcePool for DummyPool {
    type SourceIterator<'a> = core::iter::Empty<&'a mut Source>
    where
        Self: 'a;

    fn sources<'a>(&'a mut self) -> Self::SourceIterator<'a> {
        core::iter::empty()
    }
}

#[cfg(test)]
mod test {
    use core::cell::Cell;

    use crate::ExecutorMarshall;

    use super::SingleThreadMarshall;

    static S: SingleThreadMarshall = unsafe { SingleThreadMarshall::new() };

    #[test]
    fn test_single_thread_marshall() {
        let cell = Cell::new(false);
        unsafe { S.register(&cell) };
        S.wake();
        unsafe { S.unregister() };
        assert!(cell.into_inner())
    }
}
