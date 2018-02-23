use server::Server;
use stream_data::StreamData;
use std::sync::{Arc, Mutex};
use http::{Http, Request, Response};
use error::MioResult;
use self::context::{Context, Value};
use self::middleware::Middleware;
use self::group::Group;
use self::route::Route;

mod context;
mod middleware;
mod group;
mod route;

pub type Handle = Fn(&mut Context) + Send + Sync + 'static;

pub struct App {
    groups: Vec<Group>,
    begin: Vec<Middleware>,
    before: Vec<Middleware>,
    after: Vec<Middleware>,
    finish: Vec<Middleware>,
    not_found: Option<Middleware>,
}

impl App {

    pub fn new() -> App {
        App{
            groups: vec![Group::new("")],
            begin: Vec::new(),
            before: Vec::new(),
            after: Vec::new(),
            finish: Vec::new(),
            not_found: None,
        }
    }

    fn add<H>(&mut self, method: &str, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        let route = Route::new(
            method.parse().unwrap(),
            pattern.into(),
            Box::new(handle),
        );

        self.groups.get_mut(0).unwrap().routes.push(route);
        self.groups.get_mut(0).unwrap().routes.last_mut().unwrap()
    }

    pub fn get<H>(&mut self, pattern: &str, handle: H) -> &mut Route
        where H: Fn(&mut Context) + Send + Sync + 'static
    {
        self.add(&stringify!(get).to_uppercase(), pattern, handle)
    }


    pub fn run(self, url: &str) -> MioResult<()> {
        let mut server: Server = Server::new(url)?;
        server.run(Box::new(move |stream_data| {
            self.handle(stream_data);
        }))?;
        Ok(())
    }

    pub fn handle(&self, stream_data: Arc<Mutex<StreamData>>) {
        let mut http = Http::new(stream_data.clone());

        match http.decode() {
            Ok(Some(request)) => {
                let mut context = Context::new(request);
                let mut route_found = false;
                if context.next() {
                    for group in self.groups.iter() {
                        for route in group.routes.iter() {
                            if route.method() != &context.request.method {
                                continue;
                            }

                            let path = {
                                let path = context.request.path();
                                let path = path.find('?').map_or(path.as_ref(), |pos| &path[..pos]);
                                if path != "/" {
                                    path.trim_right_matches('/').to_owned()
                                } else {
                                    path.to_owned()
                                }
                            };

                            if path == route.pattern {
                                route_found = true;
                                route.execute(&mut context);
                            }

                            if !route_found {
                                if let Some(ref not_found) = self.not_found {
                                    not_found.execute(&mut context);
                                } else {
                                    context.response.status(404).from_text("Not Found").unwrap();
                                }
                            }
                        }
                    }
                }
                http.encode(context.response);
            }
            Ok(None) => {
                let response = Response::empty(100);//100 - Continue 初始的请求已经接受，客户应当继续发送请求的其余部分
                http.encode(response);
            }
            Err(err) => {
                let response = Response::empty(501);
                http.encode(response);
            }
        }
    }


}

