use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, UdpSocket},
    thread,
    time::Duration,
};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

fn main() -> Result<()> {
    let app = App::parse();
    app.exec()
}

#[derive(Debug, Parser)]
struct App {
    #[clap(long, short = 's', global = true)]
    reset_sigpipe: bool,

    /// Number of pings to accept before terminating the connection
    #[clap(long, short = 't', global = true)]
    accept_pings: Option<usize>,

    #[clap(subcommand)]
    cmd: Command,
}

impl App {
    fn exec(self) -> Result<()> {
        if self.reset_sigpipe {
            eprintln!("Resetting SIGPIPE handler");
            sigpipe::reset();
        }

        match self.cmd {
            Command::Tcp => {
                let addr = spawn_tcp_ping_thread(self.accept_pings)
                    .context("failed to spawn TCP ping thread")?;
                eprintln!("TCP ping listening on {}", addr);
                ping_tcp(&addr).context("failed to ping TCP")?;
            }
            Command::Udp => {
                let addr = spawn_udp_ping_thread(self.accept_pings)
                    .context("failed to spawn UDP ping thread")?;
                eprintln!("UDP ping listening on {}", addr);

                ping_udp(&addr).context("failed to ping UDP")?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    Tcp,
    Udp,
}

fn spawn_tcp_ping_thread(accept_pings: Option<usize>) -> Result<SocketAddr> {
    let listener = TcpListener::bind("127.0.0.1:0").context("failed to bind")?;
    let addr = listener
        .local_addr()
        .context("failed to get local address")?;

    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut buf = [0; 1024];
                    for _ in 0..accept_pings.unwrap_or(usize::MAX) {
                        match stream.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                if stream.write_all(&buf[..n]).is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    Ok(addr)
}

fn ping_tcp(addr: &SocketAddr) -> Result<()> {
    let mut stream = TcpStream::connect(addr).context("failed to connect")?;

    for n in 0.. {
        eprintln!("ping {}", n);
        stream.write_all(b"ping")?;

        let mut buf = [0; 1024];
        stream.read(&mut buf)?;
        thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}

fn spawn_udp_ping_thread(accept_pings: Option<usize>) -> Result<SocketAddr> {
    let listener = UdpSocket::bind("127.0.0.1:0").context("failed to bind")?;
    let addr = listener
        .local_addr()
        .context("failed to get local address")?;

    thread::spawn(move || {
        let mut buf = [0; 1024];
        for _ in 0..accept_pings.unwrap_or(usize::MAX) {
            match listener.recv_from(&mut buf) {
                Ok((n, addr)) => {
                    if listener.send_to(&buf[..n], addr).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    Ok(addr)
}

fn ping_udp(addr: &SocketAddr) -> Result<()> {
    // UDP is connectionless, so all we're doing here is creating a socket at some arbitrary
    // address.
    let socket = UdpSocket::bind("127.0.0.1:0").context("failed to connect")?;

    for n in 0.. {
        eprintln!("ping {}", n);
        socket.send_to(b"ping", addr)?;

        let mut buf = [0; 1024];
        socket.recv_from(&mut buf)?;
        thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}
