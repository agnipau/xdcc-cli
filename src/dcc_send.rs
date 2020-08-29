use {
    crate::xdcc::DCC_SEND,
    indicatif::{ProgressBar, ProgressStyle},
    std::{
        fs::File,
        io::{self, Read, Write},
        net::{IpAddr, Ipv4Addr, Shutdown, TcpStream},
    },
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed creating the DccSend instance")]
    Init(String),

    #[error("Failed creating the output file")]
    CreateOutputFile(io::Error),

    #[error("Failed to flush the output file")]
    FlushOutputFile(io::Error),

    #[error("Failed to shutdown the TCP stream")]
    Shutdown(io::Error),

    #[error("Failed connecting to the TCP stream")]
    Connect(io::Error),

    #[error("Failed reading from the TCP stream")]
    Read(io::Error),

    #[error("Failed writing to the TCP stream")]
    Write(io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct DccSend {
    filename: String,
    ip: IpAddr,
    port: String,
    file_size: usize,
}

impl DccSend {
    pub fn from(msg: &str) -> Result<Self> {
        let captures = DCC_SEND
            .captures(&msg)
            .ok_or_else(|| Error::Init(format!("Message {} isn't valid", msg)))?;
        let ip_number = captures[2]
            .parse::<u32>()
            .map_err(|_e| Error::Init("Failed to parse ip_number".into()))?;
        log::debug!("Created DccSend instance");
        Ok(Self {
            filename: captures[1].into(),
            ip: IpAddr::V4(Ipv4Addr::from(ip_number)),
            port: captures[3].into(),
            file_size: captures[4]
                .parse::<usize>()
                .map_err(|_e| Error::Init("Failed to parse file_size".into()))?,
        })
    }

    pub fn download_file(&self) -> Result<()> {
        let mut file = File::create(&self.filename).map_err(Error::CreateOutputFile)?;
        log::debug!("Downloading file (connecting to TPC stream)");
        let mut stream =
            TcpStream::connect(format!("{}:{}", self.ip, self.port)).map_err(Error::Connect)?;
        let mut buffer = [0; 4096];
        let mut progress = 0_usize;

        let pb = ProgressBar::new(self.file_size as u64)
            .with_style(ProgressStyle::default_bar().template(
            "{wide_bar:.cyan/blue} {bytes:.green} / {total_bytes} [{elapsed} .. {eta:.cyan} at {prefix}]",
        ));
        let started = std::time::Instant::now();
        while progress < self.file_size {
            let count = stream.read(&mut buffer).map_err(Error::Read)?;
            let _ = file.write(&buffer[..count]).map_err(Error::Write)?;
            progress += count;
            pb.set_position(progress as u64);
            let elapsed = started.elapsed().as_secs_f64();
            pb.set_prefix(&format!(
                "{}/s",
                indicatif::HumanBytes((progress as f64 / elapsed).round() as u64)
            ));
        }
        pb.finish_with_message(&format!("Downloaded at {}", self.filename));

        stream.shutdown(Shutdown::Both).map_err(Error::Shutdown)?;
        file.flush().map_err(Error::FlushOutputFile)?;
        Ok(())
    }
}
