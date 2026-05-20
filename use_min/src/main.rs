use shvessel::callback::{CommandCall, CommandCallback, CommandResult};
use shvessel::cmd;
use shvessel::vessel::Vessel;

// This Example is so minimal that it should be self explanatory
// For reference see the /use/ demo

fn main() {
    use_min();
}

fn use_min() {
    let mut vessel = Vessel::<1, 1>::new();
    let _ = vessel.register(cmd::ENVIRONMENT, CommandCallback::syn_call(env_sync));
    let args = []; // environment needs no args

    std::println!("{}", cmd::ENVIRONMENT);
    let _ = vessel.execute("env", &args);
}

fn env_sync(_call: CommandCall<'_>) -> CommandResult {
    for (name, value) in std::env::vars() {
        std::println!("{}={}", name, value);
    }
    Ok(())
}
