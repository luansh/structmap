use structmap::FromMap;
use structmap_derive::FromMap;

#[derive(Debug, FromMap)]
struct MCMonitor {
    mc0: i64,
    mc1: i64,
    mc: i64,
}

impl Default for MCMonitor {
    fn default() -> Self {
        Self {
            mc0: 0,
            mc1: 0,
            mc: 0,
        }
    }
}

fn main() {
    let mut maps = GenericMap::new();

    maps.insert(String::from("mc0"), Value::new(1024_i64));
    maps.insert(String::from("mc1"), Value::new(1024_i64));
    maps.insert(String::from("mc"), Value::new(2048_i64));

    let mc_monitor: MCMonitor = MCMonitor::from_genericmap(maps);
    println!("{:?}", mc_monitor);
}
