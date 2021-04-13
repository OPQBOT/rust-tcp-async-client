use failure::Error;
use futures::{channel::mpsc, StreamExt};
use rust_network::{chatmsg::ChatMessage, connections::Connections, Packet};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let (incoming_tx, mut incoming_rx) = mpsc::unbounded();
    let connections = Connections::new(incoming_tx);

    for _ in 0..1 {
        //创建100个 tcp client
        let id = Connections::gen_random_string(16);
        connections
            .add_client(id, "127.0.0.1:8080".parse().unwrap())
            .await
            .unwrap();
    }

    let on_receive = async {
        while let Some(packet) = incoming_rx.next().await {
            match packet {
                (c, Packet::PingRequest(pkg)) => {
                    if let Ok(res) = c.send_for_response(Packet::PingRequest(pkg)).await {
                        println!("rcv pkg conn  {:#?} {:#?}", c.client_id.read().await, res);
                    }
                }
                (_, Packet::PongResponse(pkg)) => {
                    println!("rcv pkg  {:#?}", pkg)
                }
                (c, Packet::ChatMessage(pkg)) => {
                    println!(
                        "收到到服务端端消息 to {} from {} content {}",
                        &pkg.to_user,
                        &pkg.from_user,
                        if let Ok(s) = std::str::from_utf8(&pkg.content) {
                            s.to_string()
                        } else {
                            "".to_string()
                        }
                    );

                    c.spawn_lua(Packet::ChatMessage(pkg.clone())).unwrap();
                }
            }
        }
    };

    let on_send = async {
        loop {
            // connections
            //     .send_data(Packet::ChatMessage(ChatMessage {
            //         msg_id: 77,
            //         to_user: "123".to_string(),
            //         from_user: "789".to_string(),
            //         content: "123".as_bytes().to_vec(),
            //     }))
            //     .await
            //     .unwrap();
            let msg = ChatMessage {
                msg_id: 0,
                to_user: "123".to_string(),
                from_user: "789".to_string(),
                content: Connections::gen_random_string(16).as_bytes().to_vec(),
            };

            for (_id, c) in connections.clients.read().await.iter() {
                sleep(Duration::from_millis(1000 * 20)).await;
                //println!("client id send {:#?}",id);
                let pkg = c.send_for_response(Packet::ChatMessage(msg.clone())).await;
                if pkg.is_err() {
                    println!("err {:#?}", pkg.err());
                    break;
                }
                //println!("res {:#?}", res.unwrap())
                let pkg = pkg.unwrap();
                println!(
                    "同步收到服务端端消息  当前客户端ID {}to {} from {} content {}",
                    &c.client_id.read().await,
                    &pkg.to_user,
                    &pkg.from_user,
                    if let Ok(s) = std::str::from_utf8(&pkg.content) {
                        s.to_string()
                    } else {
                        "".to_string()
                    }
                );
            }
        }
    };
    tokio::select! {
        _ = on_receive => {
            println!("write_loop() exit first")
        }
        _ = connections.run() => {
            println!("connections_loop() exit first")
        },
        _ = on_send => {
            println!("send_loop() exit first")
        }
    };
    Ok(())
}
