struct Event {
    handlers: Vec<Box<dyn Fn()>>,
}
impl Event {
    fn add_handler(&mut self, handler: impl Fn() + 'static) {
        self.handlers.push(Box::new(handler));
    }
    fn trigger(&self) {
        for handler in &self.handlers {
            handler();
        }
    }
}