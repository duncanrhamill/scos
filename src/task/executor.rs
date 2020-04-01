// ---------------------------------------------------------------------------
// USE STATEMENTS
// ---------------------------------------------------------------------------

use super::{Task, TaskId};
use alloc::{collections::{BTreeMap, VecDeque}, sync::Arc, task::Wake};
use core::task::{Waker, Context, Poll};
use crossbeam_queue::ArrayQueue;

// ---------------------------------------------------------------------------
// DATA STRUCTURES
// ---------------------------------------------------------------------------

/// An executor implementing a simple queue algorithm with waker support.
pub struct Executor {
    task_queue: VecDeque<Task>,
    waiting_tasks: BTreeMap<TaskId, Task>,
    wake_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: BTreeMap<TaskId, Waker>
}

impl Executor {

    /// Create a new instance of the executor.
    pub fn new() -> Executor {
        Executor {
            task_queue: VecDeque::new(),
            waiting_tasks: BTreeMap::new(),
            wake_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: BTreeMap::new()
        }
    }

    /// Spawn a new task in the executor.
    pub fn spawn(&mut self, task: Task) {
        self.task_queue.push_back(task)
    }

    /// Run the executor
    pub fn run(&mut self) -> ! {
        loop {
            self.wake_tasks();
            self.run_ready_tasks();
            self.sleep_if_idle();
        }
    }

    /// If the wake queue is empty sleep the CPU by calling halt.
    fn sleep_if_idle(&self) {
        if !self.wake_queue.is_empty() {
            return;
        }

        x86_64::instructions::interrupts::disable();
        if self.wake_queue.is_empty() {
            x86_64::instructions::interrupts::enable_interrupts_and_hlt();
        }
        else {
            x86_64::instructions::interrupts::enable();
        }
    }

    /// Run all ready-to-execute tasks
    fn run_ready_tasks(&mut self) {

        // While there are tasks to process in the queue
        while let Some(mut task) = self.task_queue.pop_front() {
            let task_id = task.id;

            // Check if the task id is already in the waker cache
            if !self.waker_cache.contains_key(&task_id) {
                // Insert a new waker for this task into the cache
                self.waker_cache.insert(task_id, self.create_waker(task_id));
            }

            // Get the waker for this task from the cachce
            let waker = self.waker_cache.get(&task_id)
                .expect("[EXEC-ERROR] Expected waker to be present in cache \
                    but could not find it!");
            
            // Get the context
            let mut context = Context::from_waker(waker);

            match task.poll(&mut context) {
                Poll::Ready(()) => {
                    // Task is complete, remove the waker from the cache
                    self.waker_cache.remove(&task_id);
                },
                Poll::Pending => {
                    // Add the task to the waiting tasks list
                    if self.waiting_tasks.insert(task_id, task).is_some() {
                        panic!("[EXEC-ERROR] A task with the same ID is \
                            already waiting!");
                    }
                }
            }
        }
    }

    /// Create a new waker for the particular task 
    fn create_waker(&self, task_id: TaskId) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            wake_queue: self.wake_queue.clone()
        }))
    }

    /// Handle task wakeups
    fn wake_tasks(&mut self) {
        // While there are tasks to be woken from the wake queue
        while let Ok(task_id) = self.wake_queue.pop() {
            if let Some(task) = self.waiting_tasks.remove(&task_id) {
                self.task_queue.push_back(task);
            }
        }
    }
}

/// A waker for a particular task
struct TaskWaker {
    /// The ID of the task to be woken
    task_id: TaskId,

    /// A sharted reference to the `Executor`'s wake queue
    wake_queue: Arc<ArrayQueue<TaskId>>
}

impl TaskWaker {
    /// Flag this task for waking
    fn wake_task(&self) {
        self.wake_queue.push(self.task_id)
            .expect("[EXEC-ERROR] Cannot wake task as the wake queue is full.");
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}