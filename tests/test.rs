use health_tracker::{DetectorOperation};
use uuid::Uuid;

fn unwrap_up_info(up_info: DetectorOperation) -> Vec<Uuid> {
    match up_info {
        DetectorOperation::UpInfo(up) => up,
        _ => panic!("invalid type"),
    }
}

#[cfg(test)]
mod tests {
    use health_tracker::{DetectorOperation, FailureDetectorActor};
    use actix::clock::Duration;
    use std::net::SocketAddr;
    use tokio::net::UdpSocket;
    use uuid::Uuid;

    use super::*;

    #[actix_rt::test]
    async fn data_on_wire_should_parse_with_bincode_for_single_node() {
        let delay = Duration::from_millis(20);
        let (ident, addr): (Uuid, SocketAddr) =
            (Uuid::new_v4(), "127.0.0.1:17844".parse().unwrap());
        let addresses = vec![(ident, addr)].iter().cloned().collect();
        let all_idents = vec![ident].iter().cloned().collect();

        let send_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let mut sock = UdpSocket::bind(send_addr).await.unwrap();
        let _detector = FailureDetectorActor::new(delay, &addresses, ident, all_idents).await;
        assert_eq!(
            sock.send_to(
                bincode::serialize(&DetectorOperation::UpRequest)
                    .unwrap()
                    .as_slice(),
                addr,
            )
            .await
            .unwrap(),
            4
        );

        let mut buf = [0; 256];
        let len = sock.recv(&mut buf).await.unwrap();
        let up_info = unwrap_up_info(bincode::deserialize(&buf[..len]).unwrap());

        assert_eq!(up_info.len(), 1);
        assert_eq!(up_info.get(0).unwrap(), &ident);
    }
}
