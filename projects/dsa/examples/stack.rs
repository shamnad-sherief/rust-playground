fn main() {
    let mut stack = Stack::new_with_item(String::from("hi"));
    stack.push("Hello".into());
    print!("{:?}", stack.peek())
}

struct Stack<T> {
    elements: Vec<T>,
}

impl<T> Stack<T> {
    fn new() -> Stack<T> {
        Self {
            elements: Vec::new(),
        }
    }

    fn new_with_item(item: T) -> Self {
        let vec = vec![item];
        Self { elements: vec }
    }

    fn push(&mut self, item: T) {
        self.elements.push(item);
    }

    fn pop(&mut self) -> Option<T> {
        self.elements.pop()
    }

    fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    fn peek(&self) -> Option<&T> {
        self.elements.last()
    }
}
