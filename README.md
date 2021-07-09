# atomsub

Simple PubSub Server in Rust.

It uses Polling from smol in the main thread to detect connections, read and
write. It handles subscriptions in another thread.

## Unrelated inspiration

- [Actors with Tokio](https://ryhl.io/blog/actors-with-tokio/)
- [Telnet chat](https://github.com/Darksonn/telnet-chat)
