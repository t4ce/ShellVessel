#![no_std]
#![no_main]

use core::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

use esp_backtrace as _;
use esp_hal::{
    clock::CpuClock,
    delay::Delay,
    main,
    time::{Duration, Instant},
};
use esp_println::println;
use shvessel::{
    callback::{CommandCall, CommandCallback, CommandResult},
    job::PlatformJob,
    mini_exec::{MiniExecutor, MiniTaskContext},
    vessel::Vessel,
    Command, ReturnCodes,
};

const PROVE: Command = Command::new("prove", "pv", "prove shvessel mini_exec on esp-hal");

fn proof_task(mut task: MiniTaskContext<'_>) -> Poll<CommandResult> {
    match task.state() {
        0 => {
            println!("SHVESSEL_ESP_HAL:task_spawned:{}", task.job().id);
            task.set_state(1);
            task.sleep_for(50);
            Poll::Pending
        }
        1 => {
            println!("SHVESSEL_ESP_HAL:task_woke_at:{}ms", task.now_ms());
            Poll::Ready(Ok(()))
        }
        _ => Poll::Ready(Err(ReturnCodes::Failed)),
    }
}

fn spawn_proof(mut call: CommandCall<'_>) -> Result<PlatformJob, ReturnCodes> {
    let exec = unsafe { call.context_mut::<MiniExecutor<2>>() }.ok_or(ReturnCodes::Failed)?;
    exec.spawn("esp-hal-proof", proof_task)
}

#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let _peripherals = esp_hal::init(config);
    let delay = Delay::new();

    println!("SHVESSEL_ESP_HAL:boot");

    let mut vessel = Vessel::<1, 1>::new();
    let mut exec = MiniExecutor::<2>::new();
    let callback =
        CommandCallback::asyn_call_with_context(spawn_proof, (&mut exec as *mut _) as *mut ());
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);

    match vessel.register(PROVE, callback) {
        Ok(()) => println!("SHVESSEL_ESP_HAL:registered"),
        Err(code) => {
            println!("SHVESSEL_ESP_HAL:register_error:{:?}", code);
            halt(delay);
        }
    }

    let vessel_job = match vessel.execute("prove", &[]) {
        Ok(job) => {
            println!("SHVESSEL_ESP_HAL:vessel_job:{}", job.0);
            job
        }
        Err(code) => {
            println!("SHVESSEL_ESP_HAL:execute_error:{:?}", code);
            halt(delay);
        }
    };

    loop {
        exec.advance_to(now_ms());

        match vessel.poll_job(vessel_job, &mut exec, &mut context) {
            Ok(Poll::Pending) => {}
            Ok(Poll::Ready(Ok(()))) => {
                println!("SHVESSEL_ESP_HAL:done");
                halt(delay);
            }
            Ok(Poll::Ready(Err(code))) => {
                println!("SHVESSEL_ESP_HAL:job_error:{:?}", code);
                halt(delay);
            }
            Err(code) => {
                println!("SHVESSEL_ESP_HAL:poll_error:{:?}", code);
                halt(delay);
            }
        }

        delay.delay(Duration::from_millis(1));
    }
}

fn now_ms() -> u64 {
    Instant::now().duration_since_epoch().as_millis()
}

fn halt(delay: Delay) -> ! {
    loop {
        delay.delay(Duration::from_millis(1000));
    }
}

fn noop_waker() -> Waker {
    unsafe fn clone(_: *const ()) -> RawWaker {
        raw_noop_waker()
    }

    unsafe fn wake(_: *const ()) {}

    unsafe fn wake_by_ref(_: *const ()) {}

    unsafe fn drop(_: *const ()) {}

    const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

    fn raw_noop_waker() -> RawWaker {
        RawWaker::new(core::ptr::null(), &VTABLE)
    }

    unsafe { Waker::from_raw(raw_noop_waker()) }
}
