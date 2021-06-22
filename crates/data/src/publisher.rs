// Echidna - Data

use {
    crate::*,
    codec::Codec,
    tokio::{
        net,
        task,
        io::AsyncReadExt,
        sync::Mutex,
        time,
    },
    std::{
        sync::Arc,
        net::SocketAddr,
        collections::HashMap,
        time::Duration,
    },
};

pub struct PublisherState {
    pub subs: HashMap<SubId,SubRef>,
}

pub struct Publisher {
    pub id: PubId,
    pub topic: String,
    pub socket: net::UdpSocket,
    pub address: SocketAddr,
    pub state: Mutex<PublisherState>,
}

impl Publisher {
    pub async fn new(topic: &str) -> Arc<Publisher> {

        // new ID
        let id = rand::random::<u64>();

        // open data socket
        let socket = net::UdpSocket::bind("0.0.0.0:0").await.expect("cannot create publisher socket");
        let address = socket.local_addr().expect("cannot get local address of socket");

        // create publisher
        let publisher = Arc::new(Publisher {
            id: id,
            topic: topic.to_string(),
            socket: socket,
            address: address,
            state: Mutex::new(PublisherState {
                subs: HashMap::new(),
            }),
        });

        // spawn participant receiver
        let this = Arc::clone(&publisher);
        task::spawn(async move {
            this.run_participant_connection().await;
        });

        println!("publisher {:016X} of \"{}\" running at port {}",id,topic,address.port());
        
        publisher
    }

    pub async fn run_participant_connection(self: &Arc<Publisher>) {

        loop {

            // connect to participant
            let mut stream = net::TcpStream::connect("0.0.0.0:7332").await.expect("cannot connect to participant");

            // announce publisher to participant
            send_message(&mut stream,ToPart::InitPub(self.id,PubRef {
                topic: self.topic.clone(),
            })).await;

            // receive participant messages
            let mut recv_buffer = vec![0u8; 65536];
            while let Ok(length) = stream.read(&mut recv_buffer).await {
                if length == 0 {
                    break;
                }
                if let Some((_,message)) = PartToPub::decode(&recv_buffer) {
                    match message {
                        PartToPub::Init(subs) => {
                            let mut state = self.state.lock().await;
                            state.subs = subs;
                            for (id,s) in state.subs.iter() {
                                println!("new subscriber {:016X} found at {}",id,s.address);
                            }
                        },
                        PartToPub::InitFailed => {
                            panic!("publisher initialization failed!");
                        },
                        PartToPub::NewSub(id,subscriber) => {
                            println!("subscriber {:016X} found at {}",id,subscriber.address);
                        },
                        PartToPub::DropSub(id) => {
                            println!("subscriber {:016X} lost",id);
                        },
                    }
                }
            }

            println!("participant lost...");

            // wait for a few seconds before trying again
            time::sleep(Duration::from_secs(5)).await;

            println!("attempting connection again.");
        }
    }

    pub async fn send(self: &Arc<Publisher>,_data: &[u8]) {

    }
}