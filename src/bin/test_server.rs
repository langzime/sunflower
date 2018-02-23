
extern crate sunflower;

use sunflower::app::App;

fn main() {
    let mut app = App::new();
    app.get("/", |context| {
        context.response.status(200).from_text("我是root!").unwrap();
    });
    app.get("/aaa", |context| {
        context.response.status(200).from_text("Hello world!").unwrap();
    });
    app.run("127.0.0.1:8888").unwrap();
}
