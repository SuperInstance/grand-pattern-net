use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use crate::murmur::Murmur;

/// TCP transport for reliable murmur delivery.
pub struct TcpGossip {
    pub listener: TcpListener,
    pub connections: Vec<TcpStream>,
}

impl TcpGossip {
    pub fn bind(addr: &str) -> std::io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(Self {
            listener,
            connections: Vec::new(),
        })
    }

    /// Accept pending connections (non-blocking).
    pub fn accept_connections(&mut self) -> usize {
        let mut count = 0;
        loop {
            match self.listener.accept() {
                Ok((stream, _addr)) => {
                    if stream.set_nonblocking(true).is_ok() {
                        self.connections.push(stream);
                    }
                    count += 1;
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
        count
    }

    /// Send a murmur to all connected peers.
    pub fn send_murmur(&mut self, murmur: &Murmur) -> usize {
        let data = crate::serialize::serialize_murmur(murmur);
        // Frame: 2-byte length + payload
        let mut frame = vec![0u8; 2 + data.len()];
        frame[0] = (data.len() >> 8) as u8;
        frame[1] = (data.len() & 0xFF) as u8;
        frame[2..].copy_from_slice(&data);

        let mut sent = 0;
        self.connections.retain_mut(|conn| {
            if conn.write_all(&frame).is_ok() {
                sent += 1;
                true
            } else {
                false // remove broken connections
            }
        });
        sent
    }

    /// Receive murmurs from all connected peers (non-blocking).
    pub fn receive_murmurs(&mut self) -> Vec<Murmur> {
        let mut result = Vec::new();
        let mut buf = [0u8; 256];

        self.connections.retain_mut(|conn| {
            // Try to read a framed message
            match conn.read(&mut buf[..2]) {
                Ok(2) => {
                    let len = ((buf[0] as usize) << 8) | (buf[1] as usize);
                    if len <= 254 {
                        match conn.read(&mut buf[..len]) {
                            Ok(n) if n >= 32 => {
                                let mut data = [0u8; 32];
                                data.copy_from_slice(&buf[..32]);
                                if let Some(m) = crate::serialize::deserialize_murmur(&data) {
                                    result.push(m);
                                }
                            }
                            _ => {}
                        }
                    }
                    true
                }
                Ok(0) => false, // EOF, remove
                Ok(_) => true,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => true,
                Err(_) => false,
            }
        });
        result
    }

    /// Connect to a peer (e.g., for outbound TCP).
    pub fn connect(&mut self, addr: &str) -> std::io::Result<()> {
        let stream = TcpStream::connect(addr)?;
        stream.set_nonblocking(true)?;
        self.connections.push(stream);
        Ok(())
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

/// Send a murmur over a single TCP stream (utility).
pub fn tcp_send_murmur(stream: &mut TcpStream, murmur: &Murmur) -> std::io::Result<()> {
    let data = crate::serialize::serialize_murmur(murmur);
    let mut frame = vec![0u8; 2 + data.len()];
    frame[0] = (data.len() >> 8) as u8;
    frame[1] = (data.len() & 0xFF) as u8;
    frame[2..].copy_from_slice(&data);
    stream.write_all(&frame)?;
    stream.flush()?;
    Ok(())
}

/// Receive a murmur from a single TCP stream (utility).
pub fn tcp_recv_murmur(stream: &mut TcpStream) -> std::io::Result<Option<Murmur>> {
    let mut header = [0u8; 2];
    match stream.read_exact(&mut header) {
        Ok(()) => {
            let len = ((header[0] as usize) << 8) | (header[1] as usize);
            if len > 254 {
                return Ok(None);
            }
            let mut buf = vec![0u8; len];
            stream.read_exact(&mut buf)?;
            if buf.len() >= 32 {
                let mut data = [0u8; 32];
                data.copy_from_slice(&buf[..32]);
                Ok(crate::serialize::deserialize_murmur(&data))
            } else {
                Ok(None)
            }
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
        Err(e) => Err(e),
    }
}
