use tokio::net::TcpStream;

pub struct ContainerHandle {
    // Fields for the ContainerHandle struct
    pub foo: i32,
    pub tcp_stream: Option<TcpStream>,
}
