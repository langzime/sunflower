use std::thread;
use std::result::Result;
use std::net::ToSocketAddrs;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::rc::Rc;
use std::io::{self, Write};
use std::io::ErrorKind::WouldBlock;
use std::error::Error;
use std::io::Read;
use std::fmt::{self, Formatter};
use std::convert::From;
use std::net::Shutdown;
use mio::net::{TcpListener, TcpStream};
use mio::deprecated::TryRead;
use mio::channel::{self, Receiver, Sender};
use mio::{Token, Ready, PollOpt, Poll, Events, Event, Evented};
use util::threadpool::Pool;
use stream_data::StreamData;
use server::Handle;

pub enum ConnEvent {
    Write(Token),
    Read(Token),
}

pub struct Connection {
    pub tcp_stream: TcpStream,
    pub token: Token,
    pub stream_data: Arc<Mutex<StreamData>>,
    pub closing: bool,
    tx: Sender<ConnEvent>,
    thread_pool: Rc<Pool>,
    handle: Arc<Handle>,
}

impl Connection {
    pub fn new(token: Token, tcp_stream: TcpStream, tx: Sender<ConnEvent>, thread_pool: Rc<Pool>, handle: Arc<Handle>,) -> Connection {
        Connection {
            tcp_stream: tcp_stream,
            token: token,
            stream_data: Arc::new(Mutex::new(StreamData::new(Vec::with_capacity(1024), Vec::with_capacity(1024)))),
            closing: false,
            tx: tx,
            thread_pool: thread_pool,
            handle: handle,
        }
    }

    pub fn reader(&mut self) {
        let mut stream_data = self.stream_data.lock().unwrap();

        loop {

            let mut buf = [0; 1024];

            match self.tcp_stream.read(&mut buf) {
                Ok(size) => {
                    if size == 0 {
                        self.closing = true;
                        return;
                    } else {
                        stream_data.reader.extend_from_slice(&buf[0..size]);
                        if size < 1024 {
                            break;
                        }
                    }
                }
                Err(err) => {
                    //todo 会有这个错吗
                    if let WouldBlock = err.kind() {
                        break;
                    } else {
                        self.closing = true;
                        return;
                    }
                }
            }

        }

        stream_data.remote_addr = self.tcp_stream.peer_addr().unwrap();

        let tx = self.tx.clone();
        let token = self.token.clone();

        let handle = self.handle.clone();

        let stream_data = self.stream_data.clone();

        self.thread_pool.execute(move || {

            handle(stream_data);

            tx.send(ConnEvent::Write(token)).is_ok();

        });
    }

    pub fn writer(&mut self) {
        let ref mut writer = self.stream_data.lock().unwrap().writer;

        match self.tcp_stream.write(writer) {
            Ok(size) => { ;
                if size == 0 {
                    self.closing = true;
                    return;
                }

                writer.clear();
            },
            Err(_) => {
                self.closing = true;
                return;
            }
        }

        self.tx.send(ConnEvent::Read(self.token)).is_ok();


    }
}

impl Evented for Connection {
    fn register(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
                -> io::Result<()>
    {
        self.tcp_stream.register(poll, token, interest, opts)
    }

    fn reregister(&self, poll: &Poll, token: Token, interest: Ready, opts: PollOpt)
                  -> io::Result<()>
    {
        self.tcp_stream.reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        self.tcp_stream.deregister(poll)
    }
}