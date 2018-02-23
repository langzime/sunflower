
use http::Method;
use super::Handle;
use super::context::Context;

pub struct Route {
    pub pattern: String,
    pub method: Method,
    handle: Box<Handle>,
}

impl Route {
    pub fn new(method: Method, pattern: String, handle: Box<Handle>) -> Route {
        let mut route = Route {
            pattern: pattern.clone(),
            method: method,
            handle: handle,
        };
        route
    }

    pub fn pattern(&self) -> &String {
        &self.pattern
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn execute(&self, context: &mut Context) {

        if context.next() {
            (self.handle)(context);
        }
    }
}