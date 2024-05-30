
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use futures::future::join_all;
use btc_mempool::dns_seed::get_btc_nodes;
use rand::seq::SliceRandom;
use rand::thread_rng;
use slog::Logger;
use btc_mempool::p2p::query_mempool::query_mempool;
use btc_mempool::p2p::mempool_info::MempoolInfo;


#[macro_use]
extern crate slog;
extern crate slog_term;
extern crate slog_async;
use crate::slog::Drain;

fn create_logger() -> slog::Logger {
    let decorator = slog_term::TermDecorator::new().build();
    let drain = slog_term::FullFormat::new(decorator).build().fuse();
    let drain = slog_async::Async::new(drain).build().fuse();
    slog::Logger::root(drain, o!())
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let max_nodes = 3;
    let mut rng = thread_rng();
    let mut ips: Vec<IpAddr> = get_btc_nodes();
    ips.shuffle(&mut rng);
    let selected_ips = &ips[..max_nodes];


    let timeout_duration = Duration::from_secs(5);

    let logger = create_logger();

    info!(logger, "Started. max_nodes: {}, query_timeout: {} ms", max_nodes, timeout_duration.as_millis());
    info!(logger, "Selected IPs: {:?}", selected_ips);

    let results = get_mempool_info(&logger, selected_ips.to_vec(), timeout_duration).await;
    print_report(results);
    Ok(())
}

async fn get_mempool_info(logger: &Logger, ips: Vec<IpAddr>, timeout_duration: Duration) -> Vec<Result<MempoolInfo, std::io::Error>> {
    let mut tasks = vec![];
    for ip in ips {
        tasks.push(query_mempool(logger, SocketAddr::new(ip, 8333), timeout_duration));
    }
    let results = join_all(tasks).await;
    return results;
}

fn print_mempool_info(mem_pool_info: MempoolInfo) {
    println!("IP Address: {}, Fee Filter: {:?}, Mempool Count: {:?}",
             mem_pool_info.ip_address, mem_pool_info.fee_filter, mem_pool_info.mempool_count);
}

fn print_report(results: Vec<Result<MempoolInfo, std::io::Error>>) {
    for result in results {
        match result {
            Ok(mem_pool_info) => {
                print_mempool_info(mem_pool_info);
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    }
}