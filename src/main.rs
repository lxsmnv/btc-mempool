use std::cmp::{max, min};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use futures::future::join_all;
use btc_mempool::dns_seed::get_btc_nodes;
use rand::seq::SliceRandom;
use rand::thread_rng;
use btc_mempool::p2p::query_mempool::query_mempool;
use btc_mempool::p2p::mempool_info::MempoolInfo;
use log::info;
use simple_logger::SimpleLogger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    SimpleLogger::new().init().unwrap();
    let mut rng = thread_rng();
    let mut ips: Vec<IpAddr> = get_btc_nodes();
    ips.shuffle(&mut rng);
    let max_nodes = 20;
    let selected_ips = &ips[..max_nodes];

    let timeout_duration = Duration::from_secs(5);

    info!("Started. max_nodes: {}, query_timeout: {} ms", max_nodes, timeout_duration.as_millis());
    info!("Selected IPs: {:?}", selected_ips);

    let results = get_mempool_info(selected_ips.to_vec(), timeout_duration).await;
    print_report(results);
    Ok(())
}

async fn get_mempool_info(ips: Vec<IpAddr>, timeout_duration: Duration) -> Vec<Result<MempoolInfo, std::io::Error>> {
    let mut tasks = vec![];
    for ip in ips {
        tasks.push(query_mempool(SocketAddr::new(ip, 8333), timeout_duration));
    }
    let results = join_all(tasks).await;
    return results;
}

fn print_mempool_info(mem_pool_info: MempoolInfo) {
    println!("IP Address: {}, Fee Filter: {:?}, Mempool Count: {:?}",
             mem_pool_info.ip_address, mem_pool_info.fee_filter, mem_pool_info.mempool_count);
}

fn print_report(results: Vec<Result<MempoolInfo, std::io::Error>>) {

    let mut success_count = 0;
    let mut error_count = 0;
    let mut fee_min_max = (0, 0);
    let mut fee_sum: u64 = 0;
    let mut fee_count: u64 = 0;
    let mut mempool_min_max = (0, 0);
    let mut mempool_sum: u64 = 0;
    let mut mempool_count: u64 = 0;

    fn min_max_calc(min_max: (u64, u64), value: u64) -> (u64, u64) {
        if min_max == (0, 0) {
            (value, value)
        } else {
            (min(value, min_max.0), max(value, min_max.1))
        }
    }

    for result in results {
        match result {
            Ok(mem_pool_info) => {
                print_mempool_info(mem_pool_info);
                success_count += 1;
                if let Some(fee) = mem_pool_info.fee_filter {
                    fee_min_max = min_max_calc(fee_min_max, fee);
                    fee_sum += fee;
                    fee_count += 1;
                }
                if let Some(mc) = mem_pool_info.mempool_count {
                    let mc = mc as u64;
                    mempool_min_max = min_max_calc(mempool_min_max, mc);
                    mempool_sum += mc;
                    mempool_count += 1;
                }
            }
            Err(_) => {
                error_count += 1;
            }
        }
    }
    println!("successful: {}, errors: {}", success_count, error_count);
    println!("Fee Filter: min: {}, max: {}, avg: {}", fee_min_max.0, fee_min_max.1, fee_sum / fee_count);
    println!("Mempool Count: min: {}, max: {}, avg: {}", mempool_min_max.0, mempool_min_max.1, mempool_sum / mempool_count);
}