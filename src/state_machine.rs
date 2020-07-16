pub type Receiver<Command> = dyn FnMut(&Command);
