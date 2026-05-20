use core::task::{Context, Poll};

use crate::callback::{CommandOutcome, CommandResult};
use crate::{Command, ReturnCodes, MAX_RUNNING_JOBS};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JobId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JobTimeout {
    pub value: u64,
}

impl JobTimeout {
    pub const fn new(value: u64) -> Self {
        Self { value }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlatformJob {
    pub id: u64,
}

impl PlatformJob {
    pub const fn new(id: u64) -> Self {
        Self { id }
    }
}

pub trait PlatformRuntime {
    fn poll(&mut self, job: PlatformJob, context: &mut Context<'_>) -> Poll<CommandResult>;
}

pub struct CommandJob {
    pub id: JobId,
    pub command: Command,
    pub timeout: Option<JobTimeout>,
    platform: Option<PlatformJob>,
    result: Option<CommandResult>,
}

impl CommandJob {
    pub const fn pending(
        id: JobId,
        command: Command,
        timeout: Option<JobTimeout>,
        platform: PlatformJob,
    ) -> Self {
        Self {
            id,
            command,
            timeout,
            platform: Some(platform),
            result: None,
        }
    }

    pub const fn ready(
        id: JobId,
        command: Command,
        timeout: Option<JobTimeout>,
        result: CommandResult,
    ) -> Self {
        Self {
            id,
            command,
            timeout,
            platform: None,
            result: Some(result),
        }
    }

    pub fn platform(&self) -> Option<PlatformJob> {
        self.platform
    }

    pub fn result(&self) -> Option<CommandResult> {
        self.result
    }

    pub fn poll<R: PlatformRuntime>(
        &mut self,
        runtime: &mut R,
        context: &mut Context<'_>,
    ) -> Poll<CommandResult> {
        if let Some(result) = self.result {
            return Poll::Ready(result);
        }

        let Some(platform) = self.platform else {
            return Poll::Ready(Err(ReturnCodes::Failed));
        };

        match runtime.poll(platform, context) {
            Poll::Ready(result) => {
                self.platform = None;
                self.result = Some(result);
                Poll::Ready(result)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct JobQueue<const CAPACITY: usize = MAX_RUNNING_JOBS> {
    jobs: [Option<CommandJob>; CAPACITY],
    next_id: u64,
}

impl<const CAPACITY: usize> JobQueue<CAPACITY> {
    pub fn new() -> Self {
        Self {
            jobs: [const { None }; CAPACITY],
            next_id: 1,
        }
    }

    pub fn push(&mut self, command: Command, outcome: CommandOutcome) -> Result<JobId, ReturnCodes> {
        self.push_with_timeout(command, None, outcome)
    }

    pub fn push_with_timeout(
        &mut self,
        command: Command,
        timeout: Option<JobTimeout>,
        outcome: CommandOutcome,
    ) -> Result<JobId, ReturnCodes> {
        let mut index = 0;
        while index < self.jobs.len() {
            if self.jobs[index].is_none() {
                let id = JobId(self.next_id);
                self.next_id = self.next_id.wrapping_add(1).max(1);
                self.jobs[index] = Some(match outcome {
                    CommandOutcome::Platform(platform) => CommandJob::pending(id, command, timeout, platform),
                    CommandOutcome::Ready(result) => CommandJob::ready(id, command, timeout, result),
                });
                return Ok(id);
            }
            index += 1;
        }
        Err(ReturnCodes::JobsFull)
    }

    pub fn clean(&mut self, id: JobId) -> Result<(), ReturnCodes> {
        self.take(id).map(|_| ()).ok_or(ReturnCodes::NotFound)
    }

    pub fn clean_all(&mut self) {
        let mut index = 0;
        while index < self.jobs.len() {
            self.jobs[index] = None;
            index += 1;
        }
    }

    pub fn poll<R: PlatformRuntime>(
        &mut self,
        id: JobId,
        runtime: &mut R,
        context: &mut Context<'_>,
    ) -> Result<Poll<CommandResult>, ReturnCodes> {
        let mut index = 0;
        while index < self.jobs.len() {
            if let Some(job) = self.jobs[index].as_mut() {
                if job.id == id {
                    return Ok(job.poll(runtime, context));
                }
            }
            index += 1;
        }
        Err(ReturnCodes::NotFound)
    }

    pub fn take(&mut self, id: JobId) -> Option<CommandJob> {
        let mut index = 0;
        while index < self.jobs.len() {
            if self.jobs[index]
                .as_ref()
                .map(|job| job.id == id)
                .unwrap_or(false)
            {
                return self.jobs[index].take();
            }
            index += 1;
        }
        None
    }
}

impl<const CAPACITY: usize> Default for JobQueue<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}
