use shvessel::callback::{CommandCall, CommandCallback, CommandResult};
use shvessel::cmd;
use shvessel::vessel::Vessel;

// This Example is so minimal simple sync call (no poll needed)
// For reference see the /demos_tests/vessel_demo demo

fn main() {
    let mut vessel = Vessel::<1, 1>::new();
    let _ = vessel.register(cmd::ENVIRONMENT, CommandCallback::syn_call(env_sync));
    let args = []; // environment needs no args
    let _ = vessel.execute("env", &args);
}

fn env_sync(_call: CommandCall<'_>) -> CommandResult {
    for (name, value) in std::env::vars() {
        std::println!("{}={}", name, value);
    }
    Ok(())
}
