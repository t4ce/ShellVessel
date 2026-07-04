use core::task::{Context, Poll};

use crate::ReturnCodes;
use crate::callback::CommandResult;
use crate::job::{PlatformJob, PlatformRuntime};
use crate::time::{Clock, Duration, Instant};

pub type MiniTaskPoll = fn(MiniTaskContext<'_>) -> Poll<CommandResult>;

#[derive(Clone, Copy)]
pub struct MiniTask {
    pub job: PlatformJob,
    pub name: &'static str,
    pub state: i32,
    pub value: i64,
    poll: MiniTaskPoll,
    ready: bool,
    done: bool,
    deadline_ms: Option<u64>,
    result: Option<CommandResult>,
}

impl MiniTask {
    pub const fn new(job: PlatformJob, name: &'static str, poll: MiniTaskPoll) -> Self {
        Self {
            job,
            name,
            state: 0,
            value: 0,
            poll,
            ready: true,
            done: false,
            deadline_ms: None,
            result: None,
        }
    }

    pub const fn is_ready(&self) -> bool {
        self.ready
    }

    pub const fn is_done(&self) -> bool {
        self.done
    }

    pub const fn deadline_ms(&self) -> Option<u64> {
        self.deadline_ms
    }

    pub const fn result(&self) -> Option<CommandResult> {
        self.result
    }
}

pub struct MiniTaskContext<'a> {
    task: &'a mut MiniTask,
    now_ms: u64,
}

impl<'a> MiniTaskContext<'a> {
    const fn new(task: &'a mut MiniTask, now_ms: u64) -> Self {
        Self { task, now_ms }
    }

    pub const fn job(&self) -> PlatformJob {
        self.task.job
    }

    pub const fn name(&self) -> &'static str {
        self.task.name
    }

    pub const fn now_ms(&self) -> u64 {
        self.now_ms
    }

    pub const fn now(&self) -> Instant {
        Instant::from_millis(self.now_ms)
    }

    pub const fn state(&self) -> i32 {
        self.task.state
    }

    pub const fn set_state(&mut self, state: i32) {
        self.task.state = state;
    }

    pub const fn value(&self) -> i64 {
        self.task.value
    }

    pub const fn set_value(&mut self, value: i64) {
        self.task.value = value;
    }

    pub const fn wake(&mut self) {
        self.task.ready = true;
        self.task.deadline_ms = None;
    }

    pub const fn sleep_until(&mut self, deadline_ms: u64) {
        self.task.ready = false;
        self.task.deadline_ms = Some(deadline_ms);
    }

    pub const fn sleep_until_instant(&mut self, deadline: Instant) {
        self.sleep_until(deadline.as_millis());
    }

    pub const fn sleep_for(&mut self, delay_ms: u64) {
        self.sleep_until(self.now_ms.wrapping_add(delay_ms));
    }

    pub const fn sleep_for_duration(&mut self, delay: Duration) {
        self.sleep_for(delay.as_millis());
    }
}

pub struct MiniExecutor<const CAPACITY: usize> {
    tasks: [Option<MiniTask>; CAPACITY],
    next_id: u64,
    now_ms: u64,
}

impl<const CAPACITY: usize> MiniExecutor<CAPACITY> {
    pub const fn new() -> Self {
        Self {
            tasks: [const { None }; CAPACITY],
            next_id: 1,
            now_ms: 0,
        }
    }

    pub const fn now_ms(&self) -> u64 {
        self.now_ms
    }

    pub const fn now(&self) -> Instant {
        Instant::from_millis(self.now_ms)
    }

    pub fn advance_to(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
        self.wake_expired();
    }

    pub fn advance_to_instant(&mut self, now: Instant) {
        self.advance_to(now.as_millis());
    }

    pub fn advance_from_clock<C: Clock>(&mut self, clock: &mut C) -> Instant {
        let now = clock.now();
        self.advance_to_instant(now);
        now
    }

    pub fn spawn(
        &mut self,
        name: &'static str,
        poll: MiniTaskPoll,
    ) -> Result<PlatformJob, ReturnCodes> {
        let mut index = 0;
        while index < self.tasks.len() {
            if self.tasks[index].is_none() {
                let job = PlatformJob::new(self.next_id);
                self.next_id = self.next_id.wrapping_add(1).max(1);
                self.tasks[index] = Some(MiniTask::new(job, name, poll));
                return Ok(job);
            }
            index += 1;
        }
        Err(ReturnCodes::JobsFull)
    }

    pub fn wake(&mut self, job: PlatformJob) -> Result<(), ReturnCodes> {
        let task = self.task_mut(job).ok_or(ReturnCodes::NotFound)?;
        if !task.done {
            task.ready = true;
            task.deadline_ms = None;
        }
        Ok(())
    }

    pub fn clean(&mut self, job: PlatformJob) -> Result<(), ReturnCodes> {
        let mut index = 0;
        while index < self.tasks.len() {
            if self.tasks[index]
                .as_ref()
                .map(|task| task.job == job)
                .unwrap_or(false)
            {
                self.tasks[index] = None;
                return Ok(());
            }
            index += 1;
        }
        Err(ReturnCodes::NotFound)
    }

    pub fn clean_done(&mut self) {
        let mut index = 0;
        while index < self.tasks.len() {
            if self.tasks[index]
                .as_ref()
                .map(MiniTask::is_done)
                .unwrap_or(false)
            {
                self.tasks[index] = None;
            }
            index += 1;
        }
    }

    pub fn has_live_tasks(&self) -> bool {
        let mut index = 0;
        while index < self.tasks.len() {
            if self.tasks[index]
                .as_ref()
                .map(|task| !task.done)
                .unwrap_or(false)
            {
                return true;
            }
            index += 1;
        }
        false
    }

    pub fn has_ready_tasks(&self) -> bool {
        let mut index = 0;
        while index < self.tasks.len() {
            if self.tasks[index]
                .as_ref()
                .map(|task| !task.done && task.ready)
                .unwrap_or(false)
            {
                return true;
            }
            index += 1;
        }
        false
    }

    pub fn next_deadline_ms(&self) -> Option<u64> {
        let mut best = None;
        let mut index = 0;
        while index < self.tasks.len() {
            if let Some(task) = self.tasks[index]
                && !task.done
                && let Some(deadline) = task.deadline_ms
                && best.map(|value| deadline < value).unwrap_or(true)
            {
                best = Some(deadline);
            }
            index += 1;
        }
        best
    }

    pub fn next_deadline(&self) -> Option<Instant> {
        self.next_deadline_ms().map(Instant::from_millis)
    }

    pub fn ms_until_next_deadline(&self) -> Option<u64> {
        self.next_deadline_ms()
            .map(|deadline| deadline.saturating_sub(self.now_ms))
    }

    pub fn duration_until_next_deadline(&self) -> Option<Duration> {
        self.ms_until_next_deadline().map(Duration::from_millis)
    }

    pub fn task(&self, job: PlatformJob) -> Option<&MiniTask> {
        let mut index = 0;
        while index < self.tasks.len() {
            if let Some(task) = self.tasks[index].as_ref()
                && task.job == job
            {
                return Some(task);
            }
            index += 1;
        }
        None
    }

    pub fn task_mut(&mut self, job: PlatformJob) -> Option<&mut MiniTask> {
        let mut index = 0;
        while index < self.tasks.len() {
            if self.tasks[index]
                .as_ref()
                .map(|task| task.job == job)
                .unwrap_or(false)
            {
                return self.tasks[index].as_mut();
            }
            index += 1;
        }
        None
    }

    pub fn poll_one(&mut self, job: PlatformJob) -> Poll<CommandResult> {
        self.wake_expired();

        let Some(index) = self.task_index(job) else {
            return Poll::Ready(Err(ReturnCodes::NotFound));
        };

        self.poll_index(index)
    }

    pub fn poll_ready(&mut self) -> usize {
        self.wake_expired();

        let mut polled = 0;
        let mut index = 0;
        while index < self.tasks.len() {
            if self.tasks[index]
                .as_ref()
                .map(|task| !task.done && task.ready)
                .unwrap_or(false)
            {
                let _ = self.poll_index(index);
                polled += 1;
            }
            index += 1;
        }
        polled
    }

    fn wake_expired(&mut self) {
        let mut index = 0;
        while index < self.tasks.len() {
            if let Some(task) = self.tasks[index].as_mut()
                && !task.done
                && task
                    .deadline_ms
                    .map(|deadline| deadline <= self.now_ms)
                    .unwrap_or(false)
            {
                task.ready = true;
                task.deadline_ms = None;
            }
            index += 1;
        }
    }

    fn task_index(&self, job: PlatformJob) -> Option<usize> {
        let mut index = 0;
        while index < self.tasks.len() {
            if let Some(task) = self.tasks[index]
                && task.job == job
            {
                return Some(index);
            }
            index += 1;
        }
        None
    }

    fn poll_index(&mut self, index: usize) -> Poll<CommandResult> {
        let Some(task) = self.tasks[index].as_mut() else {
            return Poll::Ready(Err(ReturnCodes::NotFound));
        };

        if let Some(result) = task.result {
            return Poll::Ready(result);
        }

        if task.done {
            return Poll::Ready(Err(ReturnCodes::Failed));
        }

        if !task.ready {
            return Poll::Pending;
        }

        task.ready = false;
        let poll = task.poll;

        match poll(MiniTaskContext::new(task, self.now_ms)) {
            Poll::Ready(result) => {
                task.done = true;
                task.result = Some(result);
                task.deadline_ms = None;
                Poll::Ready(result)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

impl<const CAPACITY: usize> PlatformRuntime for MiniExecutor<CAPACITY> {
    fn poll(&mut self, job: PlatformJob, _context: &mut Context<'_>) -> Poll<CommandResult> {
        self.poll_one(job)
    }
}

impl<const CAPACITY: usize> Default for MiniExecutor<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}
