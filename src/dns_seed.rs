
static DNS_NAMES: &[&str] = &[
    "seed.bitcoin.sipa.be.",
    "dnsseed.bluematt.me.",
    "dnsseed.bitcoin.dashjr.org.",
    "seed.bitcoinstats.com.",
    "seed.bitcoin.jonasschnelli.ch.",
    "seed.btc.petertodd.net.",
    "seed.bitcoin.sprovoost.nl.",
    "dnsseed.emzy.de.",
    "seed.bitcoin.wiz.biz.",
];

fn resolve_dns_name(dns_name: &str) -> Vec<std::net::IpAddr> {
    dns_lookup::lookup_host(dns_name).unwrap()
}

fn resolve_dns_names(dns_names: &[&str]) -> Vec<std::net::IpAddr> {
    let mut ips: Vec<std::net::IpAddr> = Vec::new();
    for dns_name in dns_names {
        let mut dns_ips: Vec<std::net::IpAddr> = resolve_dns_name(dns_name);
        ips.append(&mut dns_ips);
    }
    ips
}

pub fn get_btc_nodes() -> Vec<std::net::IpAddr> {
    resolve_dns_names(DNS_NAMES)
}