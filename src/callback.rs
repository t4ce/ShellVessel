use crate::ReturnCodes;
use crate::arg::Argument;
use crate::job::PlatformJob;

pub type CommandResult = Result<(), ReturnCodes>;
pub type CommandHandler = for<'a> fn(CommandCall<'a>) -> Result<PlatformJob, ReturnCodes>;
pub type CommandSyncHandler = for<'a> fn(CommandCall<'a>) -> CommandResult;

pub enum CommandOutcome {
    Platform(PlatformJob),
    Ready(CommandResult),
}

#[derive(Clone, Copy)]
pub struct CommandCall<'a> {
    pub arguments: &'a [Argument<'a>],
    pub context: *mut (),
}

impl<'a> CommandCall<'a> {
    pub const fn new(arguments: &'a [Argument<'a>], context: *mut ()) -> Self {
        Self { arguments, context }
    }

    /// Returns the callback context as a mutable reference.
    ///
    /// # Safety
    ///
    /// The stored context pointer must be null or valid for writes as `T`,
    /// properly aligned, and uniquely borrowed for the returned lifetime.
    pub unsafe fn context_mut<T>(&mut self) -> Option<&mut T> {
        unsafe { self.context.cast::<T>().as_mut() }
    }
}

#[derive(Clone, Copy)]
pub struct CommandCallback {
    kind: CommandCallbackKind,
    context: *mut (),
}

#[derive(Clone, Copy)]
enum CommandCallbackKind {
    Async(CommandHandler),
    Sync(CommandSyncHandler),
}

impl CommandCallback {
    pub const fn asyn_call(handler: CommandHandler) -> Self {
        Self {
            kind: CommandCallbackKind::Async(handler),
            context: core::ptr::null_mut(),
        }
    }

    pub const fn asyn_call_with_context(handler: CommandHandler, context: *mut ()) -> Self {
        Self {
            kind: CommandCallbackKind::Async(handler),
            context,
        }
    }

    pub const fn syn_call(handler: CommandSyncHandler) -> Self {
        Self {
            kind: CommandCallbackKind::Sync(handler),
            context: core::ptr::null_mut(),
        }
    }

    pub const fn syn_call_with_context(handler: CommandSyncHandler, context: *mut ()) -> Self {
        Self {
            kind: CommandCallbackKind::Sync(handler),
            context,
        }
    }

    pub const fn new(handler: CommandHandler) -> Self {
        Self::asyn_call(handler)
    }

    pub const fn with_context(handler: CommandHandler, context: *mut ()) -> Self {
        Self::asyn_call_with_context(handler, context)
    }

    pub const fn sync(handler: CommandSyncHandler) -> Self {
        Self::syn_call(handler)
    }

    pub const fn sync_with_context(handler: CommandSyncHandler, context: *mut ()) -> Self {
        Self::syn_call_with_context(handler, context)
    }

    pub fn call<'a>(self, arguments: &'a [Argument<'a>]) -> CommandOutcome {
        let call = CommandCall::new(arguments, self.context);
        match self.kind {
            CommandCallbackKind::Async(handler) => match handler(call) {
                Ok(job) => CommandOutcome::Platform(job),
                Err(code) => CommandOutcome::Ready(Err(code)),
            },
            CommandCallbackKind::Sync(handler) => CommandOutcome::Ready(handler(call)),
        }
    }
}
