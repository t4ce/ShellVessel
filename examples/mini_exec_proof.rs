#![cfg(feature = "mini_exec")]

use core::task::{Context, Poll, Waker};

use shvessel::callback::{CommandCall, CommandCallback, CommandResult};
use shvessel::job::PlatformJob;
use shvessel::mini_exec::{MiniExecutor, MiniTaskContext};
use shvessel::vessel::Vessel;
use shvessel::{Command, ReturnCodes};

const PROVE: Command = Command::new("prove", "pv", "emit mini executor proof markers");

fn proof_task(mut task: MiniTaskContext<'_>) -> Poll<CommandResult> {
    match task.state() {
        0 => {
            println!("SHVESSEL_MINI_EXEC:spawned:{}", task.job().id);
            task.set_state(1);
            task.sleep_for(25);
            Poll::Pending
        }
        1 => {
            println!("SHVESSEL_MINI_EXEC:woke_at:{}ms", task.now_ms());
            Poll::Ready(Ok(()))
        }
        _ => Poll::Ready(Err(ReturnCodes::Failed)),
    }
}

fn spawn_proof(mut call: CommandCall<'_>) -> Result<PlatformJob, ReturnCodes> {
    let exec = unsafe { call.context_mut::<MiniExecutor<2>>() }.ok_or(ReturnCodes::Failed)?;
    exec.spawn("proof", proof_task)
}

fn main() {
    let mut vessel = Vessel::<1, 1>::new();
    let mut exec = MiniExecutor::<2>::new();
    let callback =
        CommandCallback::asyn_call_with_context(spawn_proof, (&mut exec as *mut _) as *mut ());
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);

    vessel.register(PROVE, callback).unwrap();
    let job = vessel.execute("prove", &[]).unwrap();

    assert_eq!(
        vessel.poll_job(job, &mut exec, &mut context),
        Ok(Poll::Pending)
    );
    exec.advance_to(25);
    assert_eq!(
        vessel.poll_job(job, &mut exec, &mut context),
        Ok(Poll::Ready(Ok(())))
    );

    println!("SHVESSEL_MINI_EXEC:done");
}
