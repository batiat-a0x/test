mod socks;
mod utils;



use log::LevelFilter;
use net2::TcpStreamExt;
use simple_logger::SimpleLogger;
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task,
};
use utils::MAGIC_FLAG;


#[tokio::main]
async fn main() -> io::Result<()> {
	SimpleLogger::new().with_utc_timestamps().with_utc_timestamps().with_colors(true).init().unwrap();
	::log::set_max_level(LevelFilter::Info);
    let master_addr: String = "0.0.0.0:8080".into();

    log::info!("listen to : {} waiting for slave", master_addr);

    let slave_listener = match TcpListener::bind(&master_addr).await {
      Err(e) => {
          log::error!("error : {}", e);
          return Ok(());
      }
      Ok(p) => p,
  };
    let mut proxy_port_ctr = 0;
    loop {
		let socks_addr: String = "0.0.0.0:1080".into();
        //;let slave_listener = init_slave_listener.borrow();
        let (slave_stream, slave_addr) = match slave_listener.accept().await {
            Err(e) => {
                log::error!("error : {}", e);
                //return Ok::<(), E>(());
                break;
            }
            Ok(p) => p,
        };
		
        let raw_stream = slave_stream.into_std().unwrap();
        raw_stream
            .set_keepalive(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        let mut slave_stream = TcpStream::from_std(raw_stream).unwrap();

        log::info!(
            "accept slave from : {}:{}",
            slave_addr.ip(),
            slave_addr.port()
        );

        //log::info!("listen to : {}" , socks_addr);
        //let socks_addr = init_socks_addr.clone();
        tokio::spawn(async move {
            let listener = match TcpListener::bind(&socks_addr).await {
                Err(e) => {
                    log::error!("cant bind to socks_addr error : {}", e);
                    return;
                }
                Ok(p) => p,
            };

            loop {
                let (stream, _) = listener.accept().await.unwrap();

                let raw_stream = stream.into_std().unwrap();
                raw_stream
                    .set_keepalive(Some(std::time::Duration::from_secs(10)))
                    .unwrap();
                let mut stream = TcpStream::from_std(raw_stream).unwrap();

                if let Err(e) = slave_stream.write_all(&[MAGIC_FLAG[0]]).await {
                    log::error!("cant write_all to slave_stream error : {}", e);
                    break;
                    //break Ok(());
                };

                let (proxy_stream, slave_addr) = match slave_listener.accept().await {
                    Err(e) => {
                        log::error!("error : {}", e);
                        return;
                        //return Ok(());
                    }
                    Ok(p) => p,
                };

                let raw_stream = proxy_stream.into_std().unwrap();
                raw_stream
                    .set_keepalive(Some(std::time::Duration::from_secs(10)))
                    .unwrap();
                let mut proxy_stream = TcpStream::from_std(raw_stream).unwrap();

                log::info!(
                    "accept from slave : {}:{}",
                    slave_addr.ip(),
                    slave_addr.port()
                );

                task::spawn(async move {
                    let mut buf1 = [0u8; 1024];
                    let mut buf2 = [0u8; 1024];

                    loop {
                        tokio::select! {
                          a = proxy_stream.read(&mut buf1) => {

                            let len = match a {
                              Err(_) => {
                                break;
                              }
                              Ok(p) => p
                            };
                            match stream.write_all(&buf1[..len]).await {
                              Err(_) => {
                                break;
                              }
                              Ok(p) => p
                            };

                            if len == 0 {
                              break;
                            }
                          },
                          b = stream.read(&mut buf2) =>  {
                            let len = match b{
                              Err(_) => {
                                break;
                              }
                              Ok(p) => p
                            };
                            match proxy_stream.write_all(&buf2[..len]).await {
                              Err(_) => {
                                break;
                              }
                              Ok(p) => p
                            };
                            if len == 0 {
                              break;
                            }
                          },
                        }
                    }
                    log::info!(
                        "transfer [{}:{}] finished",
                        slave_addr.ip(),
                        slave_addr.port()
                    );
                });
            }
        });
    }
    Ok(())
}
