//! Types related to task management

use super::TaskContext;


//use crate::config::MAX_APP_NUM;
const MAX_SYSCALL_NUM: usize = 500;
/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// Syscall counts for the task
    pub syscall_counts: [usize; MAX_SYSCALL_NUM]
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
