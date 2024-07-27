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

    /// Number of writes to accept before terminating the connection
    #[clap(long, short = 't', global = true)]
    accept_writes: Option<usize>,

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
                let addr = spawn_tcp_listener_thread(self.accept_writes)
                    .context("failed to spawn TCP listener thread")?;
                eprintln!("TCP listening on {}", addr);
                loop_write_tcp(&addr).context("failed to write to TCP")?;
            }
            Command::Udp => {
                let addr = spawn_udp_socket_thread(self.accept_writes)
                    .context("failed to spawn UDP socket thread")?;
                eprintln!("UDP socket listening on {}", addr);

                loop_write_udp(&addr).context("failed to write to UDP")?;
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

fn spawn_tcp_listener_thread(accept_writes: Option<usize>) -> Result<SocketAddr> {
    let listener = TcpListener::bind("127.0.0.1:0").context("failed to bind")?;
    let addr = listener
        .local_addr()
        .context("failed to get local address")?;

    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let mut buf = [0; 1024];
                    for current in 0..accept_writes.unwrap_or(usize::MAX) {
                        match stream.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                eprintln!("[tcp listener] (current = {current}) read {n} bytes");
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

fn loop_write_tcp(addr: &SocketAddr) -> Result<()> {
    let mut stream = TcpStream::connect(addr).context("failed to connect")?;

    for n in 0.. {
        eprintln!("write {}", n);
        stream
            .write_all(b"ping")
            .context("failed to write to socket")?;

        thread::sleep(Duration::from_secs(1));
    }
    Ok(())
}

fn spawn_udp_socket_thread(accept_writes: Option<usize>) -> Result<SocketAddr> {
    let listener = UdpSocket::bind("127.0.0.1:0").context("failed to bind")?;
    let addr = listener
        .local_addr()
        .context("failed to get local address")?;

    thread::spawn(move || {
        let mut buf = [0; 1024];
        for current in 0..accept_writes.unwrap_or(usize::MAX) {
            match listener.recv_from(&mut buf) {
                Ok((n, addr)) => {
                    eprintln!("[udp listener] (current = {current}) read {n} bytes from {addr}");
                }
                Err(_) => break,
            }
        }
    });

    Ok(addr)
}

fn loop_write_udp(addr: &SocketAddr) -> Result<()> {
    // UDP is connectionless, so all we're doing here is creating a socket at some arbitrary
    // address.
    let socket = UdpSocket::bind("127.0.0.1:0").context("failed to connect")?;

    for n in 0.. {
        eprintln!("write {}", n);
        socket.send_to(b"ping", addr)?;

        thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}
