use std::io::{self, BufReader, BufWriter, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use super::codec::{Event, write_event};

/// Where a Wyoming server is, and how long to wait for it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Server<'a> {
    pub(crate) host: &'a str,
    pub(crate) port: u16,
    pub(crate) connect_timeout: Duration,
    pub(crate) io_timeout: Duration,
}

/// Connects to `server`, sends `requests` in order, and returns the reply stream. The whole
/// reply must arrive within `io_timeout` of the last request being sent.
pub(crate) fn exchange(
    server: &Server<'_>,
    requests: impl IntoIterator<Item = Event>,
) -> io::Result<BufReader<DeadlineReader>> {
    let stream = connect(server)?;
    let mut writer = BufWriter::new(stream.try_clone()?);
    requests
        .into_iter()
        .try_for_each(|request| write_event(&mut writer, &request))?;
    writer.flush()?;
    Ok(BufReader::new(DeadlineReader {
        stream,
        deadline: Instant::now() + server.io_timeout,
        limit: server.io_timeout,
    }))
}

pub(crate) fn connect(server: &Server<'_>) -> io::Result<TcpStream> {
    let mut last = io::Error::new(io::ErrorKind::NotFound, "the host name has no addresses");
    for address in (server.host, server.port).to_socket_addrs()? {
        match TcpStream::connect_timeout(&address, server.connect_timeout) {
            Ok(stream) => return with_timeouts(stream, server.io_timeout),
            Err(error) => last = error,
        }
    }
    Err(last)
}

fn with_timeouts(stream: TcpStream, timeout: Duration) -> io::Result<TcpStream> {
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    Ok(stream)
}

/// Reads from the socket, but never past `deadline`: every read gets only the time that is
/// left, so a server that drips bytes cannot stretch one answer beyond `limit`.
pub(crate) struct DeadlineReader {
    stream: TcpStream,
    deadline: Instant,
    limit: Duration,
}

impl DeadlineReader {
    fn timed_out(&self) -> io::Error {
        io::Error::new(
            io::ErrorKind::TimedOut,
            format!("no complete answer within {} s", self.limit.as_secs()),
        )
    }
}

impl Read for DeadlineReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let left = self.deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(self.timed_out());
        }
        self.stream.set_read_timeout(Some(left))?;
        self.stream.read(buf).map_err(|error| {
            if matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
            ) {
                self.timed_out()
            } else {
                error
            }
        })
    }
}
