use std::thread;
use std::result::Result;
use std::net::ToSocketAddrs;
use std::collections::HashMap;
use std::sync::mpsc::TryRecvError;
use std::io::Write;
use std::rc::Rc;
use std::io::{self, ErrorKind};
use std::error::Error;
use std::fmt::{self, Formatter};
use std::convert::From;
use std::net::Shutdown;
use mio::net::{TcpListener, TcpStream};
use mio::deprecated::TryRead;
use mio::channel::{self, Receiver, Sender};
use mio::{Token, Ready, PollOpt, Poll, Events, Event, Evented};
use std::sync::{Arc, Mutex};
use error::MioResult;
use util::threadpool::Pool;
use stream_data::StreamData;
use connection::{Connection, ConnEvent};

const SERVER: Token = Token(0);
const CHANNEL: Token = Token(1);

pub type Handle = Box<Fn(Arc<Mutex<StreamData>>) + Send + Sync + 'static>;

pub struct Server {
    poll: Poll,
    token: usize,
    listener: TcpListener,
    conns: HashMap<Token, Connection>,
    events: Events,
    tx: Sender<ConnEvent>,
    rx: Receiver<ConnEvent>,
    thread_pool: Rc<Pool>,
    handle: Arc<Handle>,
}

impl Server {
    pub fn new(url: &str) -> MioResult<Server> {
        let l = TcpListener::bind(&url.parse().unwrap()).unwrap();
        let (tx, rx) = channel::channel::<ConnEvent>();
        let poll = Poll::new().unwrap();
        let server = Server {
            poll,
            token: 4,
            listener: l,
            conns: HashMap::new(),
            events: Events::with_capacity(1024),
            tx,
            rx,
            thread_pool: Rc::new(Pool::new()),
            handle: Arc::new(Box::new(|_| {})),
        };
        return Ok(server)
    }

    pub fn run(&mut self, handle: Handle) -> MioResult<()> {

        self.handle = Arc::new(handle);

        //listener事件注册
        self.poll.register(&self.listener, SERVER, Ready::readable(), PollOpt::level())?;

        //数据读写事件注册 通道
        self.poll.register(&self.rx, CHANNEL, Ready::readable(), PollOpt::level())?;

        let mut events = Events::with_capacity(128);
        loop {
            self.poll.poll(&mut events, None)?;

            for event in &events {
                match event.token() {
                    SERVER => {//建立连接
                        self.token = self.token + 1;
                        let new_token = Token::from(self.token);
                        let (mut tcp_stream, _) = self.listener.accept().unwrap();
                        self.poll.register(
                            &tcp_stream, new_token,
                            Ready::readable() | Ready::hup(),
                            PollOpt::edge() | PollOpt::oneshot()
                        )?;
                        self.conns.insert(new_token, Connection::new(new_token, tcp_stream, self.tx.clone(), self.thread_pool.clone(), self.handle.clone()));
                    },
                    CHANNEL => {//StreamData读写注册
                        self.channel();
                    },
                    token => {//接入tcp_stream事件处理
                        self.connect(event, token);
                    }

                };
            }
        }
    }

    ///
    /// StreamData读写事件注册
    fn channel(&mut self) -> MioResult<()> {
        loop {
            match self.rx.try_recv() {
                Ok(event) => {
                    match event {
                        ConnEvent::Write(token) => {
                            if let Some(conn) = self.conns.get(&token) {
                                conn.reregister(
                                    &self.poll, token,
                                    Ready::writable() | Ready::hup(),
                                    PollOpt::edge() | PollOpt::oneshot()
                                )?;
                            }
                        },
                        ConnEvent::Read(token) => {
                            if let Some(conn) = self.conns.get(&token) {
                                conn.reregister(
                                    &self.poll, token,
                                    Ready::readable() | Ready::hup(),
                                    PollOpt::edge() | PollOpt::oneshot()
                                )?;
                            }
                        },
                    }
                },
                Err(err) => {
                    match err {
                        TryRecvError::Empty => {
                            break;
                        },
                        TryRecvError::Disconnected => {
                            return Err(
                                io::Error::new(ErrorKind::ConnectionAborted, err).into())
                        },
                    }
                }
            }
        }

        Ok(())
    }

    fn connect(&mut self, event: Event, token: Token) -> MioResult<()> {

        if event.readiness().is_hup() || event.readiness().is_error() {
            if let Some(conn) = self.conns.remove(&token) {
                self.poll.deregister(&conn.tcp_stream);
                return Ok(())
            }
        }

        let mut close = false;

        if event.readiness().is_readable() {
            if let Some(conn) = self.conns.get_mut(&token) {
                conn.reader();
                close = conn.closing;
            }
        }

        if event.readiness().is_writable() {
            if let Some(conn) = self.conns.get_mut(&token) {
                conn.writer();
                close = conn.closing;
            }
        }

        if close {
            if let Some(conn) = self.conns.remove(&token) {
                conn.deregister(&self.poll)?;
                conn.tcp_stream.shutdown(Shutdown::Both);
            }
        }

        Ok(())
    }
}