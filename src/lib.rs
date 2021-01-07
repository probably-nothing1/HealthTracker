use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::time::Duration;

use actix::io::SinkWrite;
use actix::{Actor, Addr, AsyncContext, Context, Message, StreamHandler};
use bytes::{Bytes, BytesMut};
use futures::stream::{SplitSink, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::UdpSocket;
use tokio_util::codec::BytesCodec;
use tokio_util::udp::UdpFramed;
use uuid::Uuid;

type SinkItem = (Bytes, SocketAddr);
type UdpSink = SplitSink<UdpFramed<BytesCodec>, SinkItem>;

pub struct FailureDetectorActor {
    id: Uuid,
    recipients: HashMap<Uuid, SocketAddr>,
    sink: SinkWrite<SinkItem, UdpSink>,
    alive_processes: Vec<Uuid>,
}

impl Actor for FailureDetectorActor {
    type Context = Context<Self>;
}

fn prepare_send_item(op: DetectorOperation, recipient_addr: SocketAddr) -> SinkItem {
    let msg = bincode::serialize(&op).unwrap();
    let msg = Bytes::from(msg);
    (msg, recipient_addr)
}

impl StreamHandler<DetectorOperationUdp> for FailureDetectorActor {
    fn handle(&mut self, item: DetectorOperationUdp, _ctx: &mut Self::Context) {
        let DetectorOperationUdp(operation, originator_addr) = item;

        match operation {
            DetectorOperation::HeartbeatRequest(originator_id) => {
                let op = DetectorOperation::HeartbeatResponse(self.id);
                let originator_addr = self.recipients.get(&originator_id).unwrap();
                let item = prepare_send_item(op, *originator_addr);
                self.sink.write(item);
            }
            DetectorOperation::HeartbeatResponse(alive_id) => {
                self.alive_processes.push(alive_id);
            }
            DetectorOperation::UpRequest => {
                let op = DetectorOperation::UpInfo(self.alive_processes.clone());
                let item = prepare_send_item(op, originator_addr);
                self.sink.write(item);
            }
            DetectorOperation::UpInfo(_) => (),
        }
    }
}

impl actix::io::WriteHandler<std::io::Error> for FailureDetectorActor {}

impl FailureDetectorActor {
    pub async fn new(
        delay: Duration,
        addresses: &HashMap<Uuid, SocketAddr>,
        ident: Uuid,
        _all_idents: HashSet<Uuid>,
    ) -> Addr<Self> {
        let addr = addresses.get(&ident).unwrap();
        let sock = UdpSocket::bind(addr).await.unwrap();

        let (sink, stream) = UdpFramed::new(sock, BytesCodec::new()).split();

        FailureDetectorActor::create(|ctx| {
            ctx.add_stream(stream.filter_map(
                |item: std::io::Result<(BytesMut, SocketAddr)>| async {
                    item.map(|(data, sender)| {
                        DetectorOperationUdp(
                            bincode::deserialize(data.as_ref())
                                .expect("Invalid format of detector operation!"),
                            sender,
                        )
                    })
                    .ok()
                },
            ));

            ctx.run_interval(delay, move |a, _| {
                a.tick();
            });

            let sink = SinkWrite::new(sink, ctx);
            let mut recipients = addresses.clone();
            recipients.remove(&ident);
            FailureDetectorActor {
                id: ident,
                sink,
                recipients,
                alive_processes: vec![ident],
            }
        })
    }

    /// Called periodically to check send broadcast and update alive processes.
    fn tick(&mut self) {
        self.alive_processes.clear();
        self.alive_processes.push(self.id);
        for recipeint in self.recipients.iter() {
            let msg = DetectorOperation::HeartbeatRequest(self.id);
            let msg = bincode::serialize(&msg).unwrap();
            let msg = Bytes::from(msg);
            let addr = recipeint.1;
            let item: SinkItem = (msg, *addr);
            self.sink.write(item);
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct DetectorOperationUdp(DetectorOperation, SocketAddr);

#[derive(Serialize, Deserialize)]
pub enum DetectorOperation {
    /// Process with uuid sends heartbeat.
    HeartbeatRequest(Uuid),
    /// Response to heartbeat, contains uuid of the receiver of HeartbeatRequest.
    HeartbeatResponse(Uuid),
    /// Request to receive information about working processes.
    UpRequest,
    /// Vector of processes which are up according to UpRequest receiver.
    UpInfo(Vec<Uuid>),
}
