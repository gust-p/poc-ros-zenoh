use std::io::Write;
use std::time::Duration;

use capnp_rpc::{RpcSystem, rpc_twoparty_capnp, twoparty};
use futures::AsyncReadExt;
use termion::event::{Event, Key};
use termion::input::MouseTerminal;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use tokio::time::sleep;

pub mod schema_capnp {
    include!("app/proto/schema_capnp.rs");
}

use schema_capnp::hello_service;
use schema_capnp::twist_service;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:7000";
    let stdin = std::io::stdin();
    let mut stdout = MouseTerminal::from(std::io::stdout().into_raw_mode().unwrap());
    write!(
        stdout,
        "{}{}q to exit.",
        termion::clear::All,
        termion::cursor::Goto(1, 1)
    )
    .unwrap();
    stdout.flush().unwrap();

    tokio::task::LocalSet::new()
        .run_until(async move {
            let stream = tokio::net::TcpStream::connect(&addr).await?;
            stream.set_nodelay(true)?;
            println!("Connected to {}", &addr);
            let (reader, writer) =
                tokio_util::compat::TokioAsyncReadCompatExt::compat(stream).split();
            let rpc_network = Box::new(twoparty::VatNetwork::new(
                futures::io::BufReader::new(reader),
                futures::io::BufWriter::new(writer),
                rpc_twoparty_capnp::Side::Client,
                Default::default(),
            ));

            let mut rpc_system = RpcSystem::new(rpc_network, None);
            let hello: hello_service::Client =
                rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);
            let twist: twist_service::Client =
                rpc_system.bootstrap(rpc_twoparty_capnp::Side::Server);

            tokio::task::spawn_local(rpc_system);
            println!("RPC System spawned");

            // let mut request = hello.do_hello_request();
            // request.get().init_data().set_msg("hello".to_string());
            //
            // request.send().promise.await?;
            //
            // println!("hello sent");

            for c in stdin.events() {
                let evt = c.unwrap();
                match evt {
                    Event::Key(Key::Char('q')) => {
                        println!("Exiting..");
                        break;
                    }
                    Event::Key(Key::Right) => {
                        let mut request_twist = twist.do_twist_request();
                        let mut twist_data = request_twist.get().init_data();
                        let mut linear = twist_data.reborrow().init_linear();
                        linear.set_x(1.0);
                        linear.set_y(0.0);
                        linear.set_z(0.0);
                        let mut angular = twist_data.reborrow().init_angular();
                        angular.set_x(0.0);
                        angular.set_y(0.0);
                        angular.set_z(1.0);
                        println!("Sending twist message");
                        request_twist.send().promise.await?;
                        println!("Twist message sent");
                        sleep(Duration::from_secs(1)).await;
                    }
                    _ => {
                        continue;
                    }
                }
            }

            Ok(())
        })
        .await
}
