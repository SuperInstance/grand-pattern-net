use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};

/// Discovery protocol — find peers via UDP multicast.

const DISCOVERY_MAGIC: &[u8; 4] = b"GPDS";
const ANNOUNCE_MAGIC: &[u8; 4] = b"GPAN";

/// Announce this node's presence on the multicast group.
pub fn announce_presence(multicast_addr: &str, port: u16, id: usize) -> std::io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let mut msg = Vec::with_capacity(12);
    msg.extend_from_slice(ANNOUNCE_MAGIC);
    msg.extend_from_slice(&(id as u64).to_le_bytes());
    let addr: SocketAddr = multicast_addr.parse()
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "bad addr"))?;
    socket.send_to(&msg, addr)?;
    Ok(())
}

/// Discover peers by listening for announcements.
/// In production this would listen on the multicast group.
/// This is a simplified version that binds to the given port.
pub fn discover_peers(multicast_addr: &str, port: u16) -> Vec<SocketAddr> {
    let mut peers = Vec::new();

    // Try to join multicast and listen briefly
    let bind_addr = format!("0.0.0.0:{}", port);
    if let Ok(socket) = UdpSocket::bind(&bind_addr) {
        let _ = socket.set_nonblocking(true);

        // Try to join multicast group
        if let Ok(multi_ip) = multicast_addr.split(':').next().unwrap_or("0.0.0.0").parse::<Ipv4Addr>() {
            let _ = socket.join_multicast_v4(&multi_ip, &Ipv4Addr::UNSPECIFIED);
        }

        let mut buf = [0u8; 64];
        // Try reading for a very short window
        for _ in 0..10 {
            match socket.recv_from(&mut buf) {
                Ok((len, addr)) => {
                    if len >= 12 && &buf[0..4] == ANNOUNCE_MAGIC {
                        peers.push(addr);
                    }
                }
                Err(_) => break,
            }
        }
    }

    peers
}

/// Mock discovery — returns fabricated peer addresses for testing.
pub fn mock_discover_peers(count: usize, base_port: u16) -> Vec<SocketAddr> {
    (0..count)
        .map(|i| SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, base_port + i as u16)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_discovery() {
        let peers = mock_discover_peers(3, 9000);
        assert_eq!(peers.len(), 3);
        assert_eq!(peers[0].port(), 9000);
        assert_eq!(peers[1].port(), 9001);
        assert_eq!(peers[2].port(), 9002);
    }

    #[test]
    fn test_announce_format() {
        // Just verify announce doesn't panic with valid input
        // (actual network delivery requires multicast)
        let _ = announce_presence("239.255.0.1:9999", 9999, 42);
    }
}
