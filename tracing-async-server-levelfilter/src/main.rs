use std::{process::exit, time::Duration};
type Error = Box<dyn std::error::Error + Send + Sync + 'static>;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpListener,
    sync::mpsc,
    sync::mpsc::Sender,
};
use tracing::{info, instrument, span, trace, Instrument, Level};
use tracing_subscriber::{filter, fmt, prelude::*, reload};

#[tokio::main]
#[instrument]
async fn main() -> Result<(), Error> {
    let (tx, mut rx) = mpsc::channel::<u32>(32);

    // Create a filter for INFO level
    let info_filter = filter::LevelFilter::INFO;

    // wrap info_filter using reload::layer
    let (info_filter, reload_handle) = reload::Layer::new(info_filter);

    tracing_subscriber::registry()
        .with(info_filter)
        .with(fmt::Layer::default())
        .try_init()?;

    let task_number = 3;
    let mut tasks = Vec::with_capacity(task_number);

    let ospf_span = span!(target: "protocol", Level::TRACE, "ospf");
    tasks.push(tokio::spawn(ospf(1).instrument(ospf_span)));

    let bgp_span = span!(target: "protocol", Level::TRACE, "bgp");
    tasks.push(tokio::spawn(bgp(2).instrument(bgp_span)));

    let socket_server_span =
        span!(target: "log_remote_control", Level::TRACE, "log_remote_control");
    tasks.push(tokio::spawn(
        socket_server(tx).instrument(socket_server_span),
    ));

    let mut outputs = Vec::with_capacity(tasks.len());
    while let Some(message) = rx.recv().await {
        println!("GOT = {}", message);
        // Update filter layer value
        if message == 1 {
            println!("Setting LOG_LEVEL to TRACE ...");
            reload_handle.modify(|filter| *filter = filter::LevelFilter::TRACE)?;
        } else {
            println!("Setting LOG_LEVEL back to INFO ...");
            reload_handle.modify(|filter| *filter = filter::LevelFilter::INFO)?;
        }
    }

    for task in tasks {
        outputs.push(task.await.unwrap());
    }

    println!("{:?}", outputs);

    Ok(())
}

#[instrument]
async fn socket_server(tx: Sender<u32>) {
    let listener = TcpListener::bind("localhost:6666").await.unwrap();
    info!(name: "socket_server", target="log_remote_control","WAITING INCOMING CONNECTION ...");

    loop {
        let (mut socket, _addr) = listener.accept().await.unwrap();
        info!(name: "socket_server", target="log_remote_control", "GOT INCOMING CONNECTION !");
        let tx = tx.clone();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();

            let mut reader = BufReader::new(reader);
            let mut line = String::new();

            loop {
                let _bytes_read = reader.read_line(&mut line).await.unwrap();
                writer.write_all(line.as_bytes()).await.unwrap();
                match line.trim_end() {
                    "1" => {
                        info!("You type 1 !");
                        tx.send(1).await.unwrap()
                    }
                    "2" => {
                        info!("You type 2 !");
                        tx.send(2).await.unwrap()
                    }
                    _ => {
                        info!("You type something !");
                        tx.send(666).await.unwrap()
                    }
                };
                if line.trim_end() == "exit" {
                    exit(0);
                }
                line.clear();
            }
        });
        info!(name: "socket_server", target="log_automation", "EXITING INCOMING CONNECTION !");
        info!(name: "socket_server", target="log_automation","WAITING INCOMING CONNECTION ...");
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

#[instrument(target = "protocol", name = "ospf")]
async fn ospf(pid: usize) {
    let mut counter: u64 = 0;
    info!(name: "ospf", target:"protocol", pid, "ENTERING OSPF ...");
    loop {
        trace!(name: "ospf", target: "protocol", pid, "This an OSPF message");
        tokio::time::sleep(Duration::from_secs(4)).await;
        counter += 1;
        if counter == 10 {
            break;
        }
    }
    info!(name: "ospf", target:"protocol", pid, "EXITING OSPF ...");
}

#[instrument(target = "protocol", name = "bgp")]
async fn bgp(pid: usize) {
    let mut counter: u64 = 0;
    info!(name: "bgp", target:"protocol",pid, "ENTERING BGP ...");
    loop {
        trace!(name: "bgp", target: "protocol", pid, "This a BGP message");
        tokio::time::sleep(Duration::from_secs(2)).await;
        counter += 1;
        if counter == 10 {
            break;
        }
    }
    info!(name: "bgp", target: "protocol", pid,"EXITING BGP ...");
}
