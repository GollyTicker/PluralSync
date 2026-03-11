pub enum LoopStreamControl<Yielded> {
    Break,
    Continue,
    Yield(Yielded),
}
