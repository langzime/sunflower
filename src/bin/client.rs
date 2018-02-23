use std::io::prelude::*;
use std::net::TcpStream;
use std::io::stdin;
use std::thread;


fn main() {
        let mut stream=TcpStream::connect(&"127.0.0.1:8888").unwrap();
        stream.set_nodelay(true).unwrap();
        stream.write("1:1493192072:dujiajiyi:192.168.0.101:98:11644bc5:0:0:\u{0}ANDROID".as_bytes()).unwrap();
        loop {
                let mut test_stream=stream.try_clone().unwrap();
                thread::spawn(move||{
                        loop{
                                let mut buffer=[0;8096];
                                let num=test_stream.read(&mut buffer[..]).unwrap();
                                if num==0{
                                        println!("连接关闭了");
                                        break;
                                }
                                print!("--{} \n", String::from_utf8_lossy(&buffer[0..num]));
                        }
                });
                let mut response=String::new();
                stdin().read_line(&mut response).ok().expect("nothing");
                //print!("{}",response);
        }


}