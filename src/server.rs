
use futures::stream::StreamExt;
use tokio::net::TcpListener;

#[derive(Debug)]
pub struct Server {
  listener: TcpListener
    
}

impl Server {
  pub async fn new(bind_addr: String, db_addr: String) -> Server {
    Server { 
      listener: TcpListener::bind(bind_addr).await.unwrap()
    }
  }

  pub async fn run(&mut self) {
    let mut incoming = self.listener.incoming();
    while let Some(conn) = incoming.next().await {
      match conn {
        Ok(mut socket) => {
          info!("Accepted connection from {:?}", socket.peer_addr());
          tokio::spawn(async move {
            let (mut reader, mut writer) = socket.split();

            match tokio::io::copy(&mut reader, &mut writer).await {
              Ok(amt) => {
                println!("wrote {} bytes", amt);
              }
              Err(err) => {
                eprintln!("IO error {:?}", err);
              }
            }
          });
        }
        Err(err) => {
            // Handle error by printing to STDOUT.
            error!("accept error = {:?}", err);
        }
      }
    }


  }
}
