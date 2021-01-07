use actix::Arbiter;
use health_tracker::{DetectorOperation, FailureDetectorActor};
use std::collections::{HashMap, HashSet};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::thread::sleep;
use std::time::Duration;
use uuid::Uuid;

fn unwrap_up_info(up_info: DetectorOperation) -> Vec<Uuid> {
    match up_info {
        DetectorOperation::UpInfo(up) => up,
        _ => panic!("invalid type"),
    }
}

fn main() {
    let delay = Duration::from_millis(1000);
    let addr1 = (
        Uuid::new_v4(),
        ("127.0.0.1", 9121)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap(),
    );
    let addr2 = (
        Uuid::new_v4(),
        ("127.0.0.1", 9122)
            .to_socket_addrs()
            .unwrap()
            .next()
            .unwrap(),
    );
    let all_idents: HashSet<Uuid> = [addr1, addr2].iter().map(|v| v.0).collect();
    let addresses: HashMap<Uuid, SocketAddr> = [addr1, addr2].iter().cloned().collect();

    let _t = std::thread::spawn(move || {
        actix::run(async move {
            let _detector_1 =
                FailureDetectorActor::new(delay, &addresses, addr1.0, all_idents.clone()).await;
            let _detector_2 =
                FailureDetectorActor::new(delay, &addresses, addr2.0, all_idents.clone()).await;
            Arbiter::local_join().await;
        })
        .unwrap();
    });

    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();

    {
        sleep(delay * 5);

        let mut buf = [0; 512];

        socket
            .send_to(
                bincode::serialize(&DetectorOperation::UpRequest)
                    .unwrap()
                    .as_slice(),
                &addr1.1,
            )
            .expect("cannot send?");

        let len = socket.recv(&mut buf).unwrap();
        let up_info =
            unwrap_up_info(bincode::deserialize(&buf[..len]).expect("Invalid format of up info!"));
        println!("Up according to first process: {:?}", up_info);

        sleep(delay * 5);

        socket
            .send_to(
                bincode::serialize(&DetectorOperation::UpRequest)
                    .unwrap()
                    .as_slice(),
                &addr2.1,
            )
            .expect("cannot send?");

        let len = socket.recv(&mut buf).unwrap();
        let up_info =
            unwrap_up_info(bincode::deserialize(&buf[..len]).expect("Invalid format of up info!"));
        println!("Up according to second process: {:?}", up_info);
    }
}
