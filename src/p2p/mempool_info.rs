use std::net::IpAddr;
#[derive(Debug)]
pub struct MempoolInfo {
    pub ip_address: IpAddr,
    pub fee_filter: Option<u64>,
    pub mempool_count: Option<usize>
}

impl MempoolInfo {
    pub fn new(ip_address: IpAddr) -> Self {
        MempoolInfo {
            ip_address,
            fee_filter: None,
            mempool_count: None,
        }
    }

    pub fn set_fee_filter(&mut self, fee_filter: u64) {
        self.fee_filter = Some(fee_filter);
    }

    pub fn set_mempool_count(&mut self, mempool_count: usize) {
        self.mempool_count = Some(mempool_count);
    }

    pub fn update_mempool_count(&mut self, value: usize) {
        if let Some(count) = self.mempool_count {
            self.mempool_count = Some(count + value);
        } else {
            self.mempool_count = Some(value);
        }
    }
}