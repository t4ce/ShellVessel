#![cfg(feature = "mini_exec")]

use core::task::{Context, Poll, Waker};

use shvessel::callback::{CommandCall, CommandCallback, CommandResult};
use shvessel::job::PlatformJob;
use shvessel::mini_exec::{MiniExecutor, MiniTaskContext};
use shvessel::vessel::Vessel;
use shvessel::{Command, ReturnCodes};

const DELAY: Command = Command::new("delay", "dl", "complete after one executor wake");

fn delay_task(mut task: MiniTaskContext<'_>) -> Poll<CommandResult> {
    match task.state() {
        0 => {
            task.set_state(1);
            task.sleep_for(10);
            Poll::Pending
        }
        1 => Poll::Ready(Ok(())),
        _ => Poll::Ready(Err(ReturnCodes::Failed)),
    }
}

fn counting_task(mut task: MiniTaskContext<'_>) -> Poll<CommandResult> {
    let value = task.value() + 1;
    task.set_value(value);

    if value < 3 {
        task.wake();
        Poll::Pending
    } else {
        Poll::Ready(Ok(()))
    }
}

fn spawn_delay(mut call: CommandCall<'_>) -> Result<PlatformJob, ReturnCodes> {
    let exec = unsafe { call.context_mut::<MiniExecutor<4>>() }.ok_or(ReturnCodes::Failed)?;
    exec.spawn("delay", delay_task)
}

#[test]
fn vessel_polls_mini_executor_jobs_to_completion() {
    let mut vessel = Vessel::<1, 1>::new();
    let mut exec = MiniExecutor::<4>::new();
    let callback =
        CommandCallback::asyn_call_with_context(spawn_delay, (&mut exec as *mut _) as *mut ());
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    vessel.register(DELAY, callback).unwrap();
    let vessel_job = vessel.execute("delay", &[]).unwrap();

    assert_eq!(
        vessel.poll_job(vessel_job, &mut exec, &mut context),
        Ok(Poll::Pending)
    );
    assert_eq!(exec.ms_until_next_deadline(), Some(10));

    exec.advance_to(9);
    assert_eq!(
        vessel.poll_job(vessel_job, &mut exec, &mut context),
        Ok(Poll::Pending)
    );

    exec.advance_to(10);
    assert_eq!(
        vessel.poll_job(vessel_job, &mut exec, &mut context),
        Ok(Poll::Ready(Ok(())))
    );
}

#[test]
fn ready_tasks_can_be_polled_without_a_vessel() {
    let mut exec = MiniExecutor::<2>::new();
    let job = exec.spawn("count", counting_task).unwrap();

    assert!(exec.has_ready_tasks());
    assert_eq!(exec.poll_ready(), 1);
    assert_eq!(exec.poll_ready(), 1);
    assert_eq!(exec.poll_one(job), Poll::Ready(Ok(())));
    assert!(!exec.has_live_tasks());
}

#[test]
fn capacity_and_missing_jobs_report_return_codes() {
    let mut exec = MiniExecutor::<1>::new();
    let job = exec.spawn("count", counting_task).unwrap();

    assert_eq!(
        exec.spawn("extra", counting_task),
        Err(ReturnCodes::JobsFull)
    );
    assert_eq!(
        exec.clean(PlatformJob::new(999)),
        Err(ReturnCodes::NotFound)
    );
    assert_eq!(exec.clean(job), Ok(()));
    assert_eq!(exec.poll_one(job), Poll::Ready(Err(ReturnCodes::NotFound)));
}
