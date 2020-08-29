use {
    crate::{
        dcc_send::{self, DccSend},
        packs_ranges::PacksRanges,
    },
    lazy_static::lazy_static,
    rand::Rng,
    regex::Regex,
    std::{
        io::{self, Read, Write},
        net::{Shutdown, TcpStream},
        thread,
        time::Duration,
    },
};

lazy_static! {
    static ref USERNAME_VALID_CHARS: String = (b'a'..=b'z').map(|c| c as char).collect::<String>();
    static ref PING: Regex = Regex::new(r#"PING :\d+"#).unwrap();
    static ref JOIN: Regex = Regex::new(r#"JOIN :#.*"#).unwrap();
    static ref PRIVMSG: Regex = Regex::new(r#"PRIVMSG.*"#).unwrap();
    pub(crate) static ref DCC_SEND: Regex =
        Regex::new(r#"DCC SEND "?(.*)"? (\d+) (\d+) (\d+)"#).unwrap();
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed creating the Xdcc instance")]
    Init(String),

    #[error("Failed establishing a connection to the server")]
    Connect(io::Error),

    #[error("Failed writing to the TPC stream")]
    Write(io::Error),

    #[error("Failed reading from the TPC stream")]
    Read(io::Error),

    #[error("Failed to shutdown the TPC stream")]
    Shutdown(io::Error),

    #[error(transparent)]
    DccSend(dcc_send::Error),

    #[error("Failed to join thread")]
    JoinThread,
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Xdcc {
    pub nick: String,
    pub server: String,
    pub bot: String,
    pub channel: String,
    pub packs_ranges: PacksRanges,
    pub req_timeout: Option<Duration>,
    stream: Option<TcpStream>,
}

impl Xdcc {
    pub fn new(
        nick: Option<&str>,
        server: &str,
        port: u16,
        bot: &str,
        channel: &str,
        packs_ranges: &PacksRanges,
        req_timeout: Duration,
    ) -> Result<Self> {
        let rand_nick = Self::gen_rand_nick();
        let nick = nick.unwrap_or(&rand_nick);
        if nick.is_empty() {
            return Err(Error::Init("Empty nickname".into()));
        }

        if server.is_empty() {
            return Err(Error::Init("Empty server".into()));
        }

        if bot.is_empty() {
            return Err(Error::Init("Empty bot".into()));
        }

        if packs_ranges.0.is_empty() {
            return Err(Error::Init("At least one pack range is needed".into()));
        }

        let channel = {
            if let Some(fst) = channel.chars().next() {
                if fst == '#' {
                    &channel[1..]
                } else {
                    channel
                }
            } else {
                channel
            }
        };

        log::debug!("Created new Xdcc instance");
        Ok(Self {
            nick: nick.into(),
            server: format!("{}:{}", server, port),
            bot: bot.into(),
            channel: channel.into(),
            packs_ranges: packs_ranges.clone(),
            stream: None,
            req_timeout: if req_timeout.as_secs_f64() == 0.0 {
                None
            } else {
                Some(req_timeout)
            },
        })
    }

    fn log_in(&mut self) -> Result<()> {
        self.disconnect(None)?;
        log::debug!("Connecting to TPC stream at {}", self.server);
        let stream = TcpStream::connect(&self.server).map_err(Error::Connect)?;
        stream.set_read_timeout(self.req_timeout).unwrap();
        self.stream = Some(stream);
        self.write(format!("NICK {}\r\n", self.nick).as_bytes())?;
        self.write(format!("USER {0} 0 * {0}\r\n", self.nick).as_bytes())?;
        Ok(())
    }

    fn gen_rand_nick() -> String {
        let mut rng = rand::thread_rng();
        let len = (*USERNAME_VALID_CHARS).len();
        let mut username = String::new();
        for _ in 0..8 {
            let r = rng.gen_range(0, len);
            let next_char = USERNAME_VALID_CHARS.chars().nth(r).unwrap();
            username.push(next_char);
        }
        username
    }

    fn disconnect(&mut self, msg: Option<&str>) -> Result<()> {
        if let Some(mut stream) = self.stream.take() {
            log::debug!("Quitting from IRC channel");
            let _ = stream
                .write(
                    format!(
                        "QUIT :{}\r\n",
                        match msg {
                            Some(msg) => msg.to_owned(),
                            None => "".into(),
                        }
                    )
                    .as_bytes(),
                )
                .map_err(Error::Write)?;
            log::debug!("Shutting down TPC stream");
            stream.shutdown(Shutdown::Both).map_err(Error::Shutdown)?;
        }
        Ok(())
    }

    fn read_next_message(&mut self, msg_buffer: &mut String) -> Result<Option<String>> {
        if let Some(ref mut stream) = self.stream {
            let mut buffer = [0; 4];
            while !msg_buffer.contains('\n') {
                let count = stream.read(&mut buffer).map_err(Error::Read)?;
                msg_buffer.push_str(std::str::from_utf8(&buffer[..count]).unwrap_or_default());
            }
            let endline_offset = msg_buffer.find('\n').unwrap() + 1;
            let message = msg_buffer.get(..endline_offset).unwrap().to_string();
            msg_buffer.replace_range(..endline_offset, "");
            Ok(Some(message))
        } else {
            log::debug!("Can't read message. TPC stream is None");
            Ok(None)
        }
    }

    fn write(&mut self, buf: &[u8]) -> Result<()> {
        if let Some(ref mut stream) = self.stream {
            log::debug!("Writing {} bytes to TPC stream", buf.len());
            let _ = stream.write(buf).map_err(Error::Write)?;
        }
        Ok(())
    }

    pub fn download(&mut self) -> Result<()> {
        let mut download_handles = Vec::new();
        let mut has_joined = false;
        self.log_in()?;

        let mut msg_buffer = String::new();
        while download_handles.len() < self.packs_ranges.0.len() {
            if let Some(msg) = self.read_next_message(&mut msg_buffer)? {
                log::debug!("Received message: {}", msg);

                if PING.is_match(&msg) {
                    let pong = msg.replace("PING", "PONG");
                    self.write(pong.as_bytes())?;
                    if !has_joined {
                        let req = format!("JOIN #{}\r\n", self.channel);
                        log::debug!("Sending {}", req);
                        self.write(req.as_bytes())?;
                        has_joined = true;
                    }
                } else if JOIN.is_match(&msg) {
                    for range in self.packs_ranges.0.clone() {
                        let req = format!("PRIVMSG {} :xdcc send #{}\r\n", self.bot, range.start());
                        log::debug!("Sending {}", req);
                        self.write(req.as_bytes())?;
                    }
                } else if DCC_SEND.is_match(&msg) {
                    let request = DccSend::from(&msg).map_err(Error::DccSend)?;
                    let handle = thread::spawn(move || request.download_file());
                    download_handles.push(handle);
                } else if PRIVMSG.is_match(&msg) && !has_joined {
                    let req = format!("JOIN #{}\r\n", self.channel);
                    log::debug!("Sending {}", req);
                    self.write(req.as_bytes())?;
                    has_joined = true;
                }
            }
        }

        self.disconnect(None)?;
        for handle in download_handles {
            handle
                .join()
                .map_err(|_e| Error::JoinThread)?
                .map_err(Error::DccSend)?;
        }

        Ok(())
    }
}

impl Drop for Xdcc {
    fn drop(&mut self) {
        if self.disconnect(None).is_err() {
            log::error!("Failed to disconnect");
        }
    }
}
