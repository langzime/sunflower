use std::sync::{Arc, Mutex};

use std::io::Write;
use std::str::FromStr;

use httparse;

use stream_data::StreamData;
use error::MioResult;
use error::MioError;

pub use self::request::Request;
pub use self::response::Response;
pub use self::http_code::StatusCode;
pub use self::http_method::Method;
use self::http_date::HTTPDate;

mod http_code;
mod http_date;
mod http_method;
mod request;
mod response;

pub struct Http {
    stream_data: Arc<Mutex<StreamData>>,
}

impl Http {
    pub fn new(stream_data: Arc<Mutex<StreamData>>) -> Http {
        Http {
            stream_data: stream_data,
        }
    }

    pub fn decode(&mut self) -> MioResult<Option<Request>> {
        let mut stream_data = self.stream_data.lock().unwrap();

        let (method, path, headers, amt) = {
            let mut headers = [httparse::EMPTY_HEADER; 24];
            let mut req = httparse::Request::new(&mut headers);
            let res = req.parse(&mut stream_data.reader)?;

            let amt = match res {
                httparse::Status::Complete(amt) => amt,
                httparse::Status::Partial => return Err(MioError::Error("Http paser error".to_owned()))
            };

            let method = req.method.unwrap().to_owned();
            let path = req.path.unwrap().to_owned();
            let headers = req.headers.iter().map(|h| (h.name.to_owned(), String::from_utf8_lossy(h.value).to_string())).collect();

            (method, path, headers, amt)
        };

        let remote_addr = stream_data.remote_addr();

        let mut request = Request::new(
            method.parse().unwrap(),
            path,
            headers,
            remote_addr,
            Vec::new()
        );

        if let Some(len) = request.get_header("Content-Length") {
            let len: usize = usize::from_str(&len)?;
            if len > stream_data.reader.len() - amt {
                return Ok(None)
            }
        }

        request.data = stream_data.reader.split_off(amt);
        stream_data.reader.clear();

        Ok(Some(request))
    }

    pub fn encode(&mut self, response: Response) {
        let mut stream_data = self.stream_data.lock().unwrap();

        write!(stream_data, "HTTP/1.1 {} {}\r\n", response.status_code.0, response.status_code.default_reason_phrase()).unwrap();
        write!(stream_data, "Data: {}\r\n", HTTPDate::new().to_string()).unwrap();
        write!(stream_data, "Server: Webserver\r\n").unwrap();

        if let Some(data_length) = response.data_length {
            write!(stream_data, "Content-Length: {}\r\n", data_length).unwrap();
        }

        for (key, value) in response.headers {
            write!(stream_data, "{}: {}\r\n", key, value).unwrap();
        }

        write!(stream_data, "\r\n").unwrap();

        stream_data.write(&response.data).unwrap();
    }
}
