pub type StateMachineReceiver<Command> = dyn Fn(Command);
