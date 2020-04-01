// ---------------------------------------------------------------------------
// MODULES
// ---------------------------------------------------------------------------

pub mod executor;
pub mod keyboard;

// ---------------------------------------------------------------------------
// USE STATEMENTS
// ---------------------------------------------------------------------------

use core::{future:: Future, pin::Pin, task::{Poll, Context}};
use core::sync::atomic::{AtomicU64, Ordering};
use alloc::boxed::Box;

// ---------------------------------------------------------------------------
// DATA STRUCTURES
// ---------------------------------------------------------------------------

/// Task ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct TaskId(u64);

impl TaskId {
    fn new() -> TaskId {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// A task object which contains a future.
pub struct Task {
    id: TaskId,
    future: Pin<Box<dyn Future<Output = ()>>>,
}

impl Task {

    /// Createte a new task from the contained future.
    pub fn new(future: impl Future<Output = ()> + 'static) -> Task {
        Task {
            id: TaskId::new(),
            future: Box::pin(future)
        }
    }

    /// Poll the contained future using the given context.
    fn poll(&mut self, context: &mut Context) -> Poll<()> {
        self.future.as_mut().poll(context)
    }
}