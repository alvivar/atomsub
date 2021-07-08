use std::{
    collections::HashMap,
    io::Write,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
};

use polling::{Event, Poller};

use crate::conn::Connection;

pub enum Cmd {
    Add(String, usize),
    Del(String, usize),
    Call(String, String),
}

pub struct Subs {
    registry: HashMap<String, Vec<usize>>,
    write_map: Arc<Mutex<HashMap<usize, Connection>>>,
    poller: Arc<Poller>,
    pub tx: Sender<Cmd>,
    rx: Receiver<Cmd>,
}

impl Subs {
    pub fn new(write_map: Arc<Mutex<HashMap<usize, Connection>>>, poller: Arc<Poller>) -> Subs {
        let registry = HashMap::<String, Vec<usize>>::new();
        let (tx, rx) = channel::<Cmd>();

        Subs {
            registry,
            write_map,
            poller,
            tx,
            rx,
        }
    }

    pub fn handle(&mut self) {
        loop {
            match self.rx.recv() {
                Ok(Cmd::Add(key, id)) => {
                    println!("Sub Add");

                    let subs = self.registry.entry(key).or_insert_with(Vec::new);

                    if subs.iter().any(|x| x == &id) {
                        continue;
                    }

                    subs.push(id)
                }

                Ok(Cmd::Del(key, id)) => {
                    println!("Sub Del");
                    let subs = self.registry.entry(key).or_insert_with(Vec::new);
                    subs.retain(|x| x != &id);
                }

                Ok(Cmd::Call(key, value)) => {
                    println!("Sub Call");
                    if let Some(subs) = self.registry.get(&key) {
                        let mut write_map = self.write_map.lock().unwrap();
                        for id in subs {
                            if let Some(conn) = write_map.get_mut(id) {
                                println!("Sub Write");
                                let msg = format!("{} {}", key, value);

                                match conn.socket.write(msg.as_bytes()) {
                                    Ok(_) => {
                                        println!("Sub writing {:?}", conn.cache);
                                        self.poller
                                            .modify(&conn.socket, Event::writable(conn.id))
                                            .unwrap();
                                    }
                                    Err(err) => println!(
                                        "Connection #{} lost, sub failed write: {}",
                                        conn.id, err
                                    ),
                                }
                            }
                        }
                    }
                }

                Err(err) => panic!("The subs channel failed: {}", err),
            }
        }
    }
}
