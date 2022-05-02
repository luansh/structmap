use structmap::{ToMap, value::Value};
use structmap_derive::ToMap;
use std::collections::BTreeMap;

#[derive(Debug, Default, ToMap)]
struct MCMonitor {
    mc0: i64,
    mc1: i64,
    #[rename(name = "MCs")]
    mc: i64,
}

fn main() {
    let mc_monitor: MCMonitor = MCMonitor { mc0:1024_i64, mc1:1024_i64, mc:2048_i64 };
    let maps: BTreeMap<String, Value> = MCMonitor::to_genericmap(mc_monitor);
    println!("{:?}", maps);
    println!("{:?}", maps.get("MCs").unwrap().i64().unwrap());
}
