use std::{
    collections::HashMap,
    io::{self, Read, Write},
    net::TcpListener,
};

use polling::{Event, Poller};

mod conn;

use conn::Connection;

fn main() -> io::Result<()> {
    // The server and the smol Poller.
    let server = TcpListener::bind("0.0.0.0:1984")?;
    server.set_nonblocking(true)?;

    let poller = Poller::new()?;
    poller.add(&server, Event::readable(0))?;

    let mut connections = HashMap::<usize, Connection>::new();

    // Connections and events via smol Poller.
    let mut id: usize = 1;
    let mut events = Vec::new();

    loop {
        events.clear();
        poller.wait(&mut events, None)?;

        for ev in &events {
            match ev.key {
                0 => {
                    let (socket, addr) = server.accept()?;
                    socket.set_nonblocking(true)?;

                    println!("Connection #{} from {}", id, addr);

                    // Register the reading socket for reading events, and save it.
                    poller.add(&socket, Event::readable(id))?;
                    connections.insert(id, Connection::new(id, socket, addr));

                    // One more connection.
                    id += 1;

                    // The server continues listening for more clients, always 0.
                    poller.modify(&server, Event::readable(0))?;
                }

                id => {
                    if let Some(conn) = connections.get_mut(&id) {
                        if ev.readable {
                            handle_reading(conn);
                            poller.modify(&conn.socket, Event::writable(id))?;
                        } else if ev.writable {
                            handle_writing(conn);
                            poller.modify(&conn.socket, Event::readable(id))?;
                        }

                        // Forget it, it died.
                        if conn.closed {
                            poller.delete(&conn.socket)?;
                            connections.remove(&id);
                        }
                    }
                }
            }
        }
    }
}

fn handle_reading(conn: &mut Connection) {
    conn.cache = match read(conn) {
        Ok(data) => data,
        Err(err) => {
            println!("Connection #{} broken, failed read: {}", conn.id, err);
            conn.closed = true;
            return;
        }
    };
}

fn handle_writing(conn: &mut Connection) {
    match conn.socket.write(&conn.cache) {
        Ok(_) => println!("Writing {:?}", conn.cache),
        Err(err) => println!("Connection #{} lost, failed write: {}", conn.id, err),
    }
}

fn read(conn: &mut Connection) -> io::Result<Vec<u8>> {
    let mut received = vec![0; 1024 * 4];
    let mut bytes_read = 0;

    loop {
        match conn.socket.read(&mut received[bytes_read..]) {
            Ok(0) => {
                // Reading 0 bytes means the other side has closed the
                // connection or is done writing, then so are we.
                return Err(io::Error::new(io::ErrorKind::BrokenPipe, "0 bytes read"));
            }

            Ok(n) => {
                bytes_read += n;
                if bytes_read == received.len() {
                    received.resize(received.len() + 1024, 0);
                }
            }

            // Would block "errors" are the OS's way of saying that the
            // connection is not actually ready to perform this I/O operation.
            // @todo Wondering if this should be a panic instead.
            Err(ref err) if would_block(err) => break,

            Err(ref err) if interrupted(err) => continue,

            // Other errors we'll consider fatal.
            Err(err) => return Err(err),
        }
    }

    // let received_data = &received_data[..bytes_read]; // @doubt Using this
    // slice thing and returning with into() versus using the resize? Hm.

    received.resize(bytes_read, 0);

    Ok(received)
}

fn would_block(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::WouldBlock
}

fn interrupted(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::Interrupted
}
