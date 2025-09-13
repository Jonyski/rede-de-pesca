use async_channel::{Receiver, Sender, unbounded};
use async_dup::Arc;
use clap::Parser;
use smol::{Async, Unblock, future, io, prelude::*};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Mutex;

/// Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Flag saying if the peer is the first one in the network
    #[arg(short, long)]
    first: bool,
    /// List of client addresses to connect to
    #[arg(short, long, value_delimiter = ',', value_parser = parse_addr)]
    peers: Vec<SocketAddr>,
    /// Address to bind the server
    #[arg(short, long, value_parser = parse_addr)]
    bind: SocketAddr,
}

/// Parse and validate the bind address
fn parse_addr(s: &str) -> Result<SocketAddr, String> {
    if let Ok(addr) = s.parse::<SocketAddr>() {
        return Ok(addr);
    }
    Err(format!("Invalid bind/peer address: {}", s))
}

/// An event on the chat server
enum Event {
    /// A client has joined
    Join(SocketAddr, Arc<Async<TcpStream>>),
    /// A client has left.
    Leave(SocketAddr),
    /// A client sent a message
    Message(SocketAddr, String),
}

// Corrected `dispatch` function
async fn dispatch(
    streams: Arc<Mutex<Vec<Arc<Async<TcpStream>>>>>,
    receiver: Receiver<Event>,
) -> io::Result<()> {
    // Receive incoming events.
    while let Ok(event) = receiver.recv().await {
        // Lock the mutex to get mutable access to the streams vector
        let mut streams = streams.lock().unwrap();

        let output = match event {
            Event::Join(addr, stream) => {
                streams.push(stream);
                format!("{} has joined\n", addr)
            }
            Event::Leave(addr) => {
                streams.retain(|s| s.get_ref().peer_addr().unwrap() != addr);
                format!("{} has left\n", addr)
            }
            Event::Message(addr, msg) => format!("{} says: {}\n", addr, msg),
        };

        print!("{}", output);
    }
    Ok(())
}

/// Reads messages from the client and forwards them to the dispatcher task
async fn read_messages(sender: Sender<Event>, client: Arc<Async<TcpStream>>) -> io::Result<()> {
    let addr = client.get_ref().peer_addr()?;
    let mut lines = io::BufReader::new(client).lines();

    while let Some(line) = lines.next().await {
        let line = line?;
        sender.send(Event::Message(addr, line)).await.ok();
    }
    Ok(())
}

async fn bind_stream_io(stream: Arc<Async<TcpStream>>) -> io::Result<()> {
    let stdin = Unblock::new(std::io::stdin());
    let mut stdout = Unblock::new(std::io::stdout());

    future::race(
        async {
            // Get a mutable reference to the inner stream
            let res = io::copy(stdin, &mut &*stream).await;
            println!("Quit!");
            res
        },
        async {
            // Get a mutable reference to the inner stream
            let res = io::copy(&mut &*stream, &mut stdout).await;
            println!("Peer disconnected!");
            res
        },
    )
    .await?;
    Ok(())
}

#[allow(dead_code)]
async fn client_side(streams: Arc<Mutex<Vec<Arc<Async<TcpStream>>>>>) -> io::Result<()> {
    smol::block_on(async {
        println!("Connected to the fishing network :)");
        println!("Type a message and hit enter!\n");

        for stream in streams.lock().unwrap().iter() {
            smol::spawn(bind_stream_io(stream.clone())).detach();
        }
        Ok(())
    })
}

// Server main
#[allow(unused_variables, unused_mut)]
fn main() -> io::Result<()> {
    smol::block_on(async {
        let args = Args::parse();
        let listener: Async<TcpListener> = if args.first {
            Async::<TcpListener>::bind(([127, 0, 0, 1], 6000))?
        } else {
            Async::<TcpListener>::bind(args.bind)?
        };

        let mut initial_streams: Vec<Arc<Async<TcpStream>>> = vec![];
        for peer_addr in args.peers {
            if let Ok(stream) = Async::<TcpStream>::connect(peer_addr).await {
                initial_streams.push(Arc::new(stream));
            }
        }

        let streams = Arc::new(Mutex::new(initial_streams));

        println!("Listening on {}", listener.get_ref().local_addr()?);
        println!("You can fish now!\n");

        let (sender, receiver) = unbounded();
        smol::spawn(dispatch(streams.clone(), receiver)).detach();
        smol::spawn(client_side(streams.clone())).detach();

        loop {
            let (stream, addr) = listener.accept().await?;
            let client = Arc::new(stream);
            let sender = sender.clone();

            smol::spawn(async move {
                sender.send(Event::Join(addr, client.clone())).await.ok();
                read_messages(sender.clone(), client).await.ok();
                sender.send(Event::Leave(addr)).await.ok();
            })
            .detach();
        }
    })
}
