use ::anyhow::anyhow;
use ::anyhow::Context;
use ::anyhow::Result;
use ::lazy_static::lazy_static;
use ::portpicker::pick_unused_port;
use ::std::collections::HashSet;
use ::std::sync::Mutex;

const MAX_TRIES: u32 = 10;

lazy_static! {
    static ref PORTS_IN_USE: Mutex<HashSet<u16>> = Mutex::new(HashSet::new());
}

#[derive(Debug)]
pub struct ReservedPort {
    port: u16,
}

impl ReservedPort {
    pub fn reserve_port(port: u16) -> Result<Self> {
        let mut ports = PORTS_IN_USE
            .lock()
            .map_err(|_| anyhow!("Failed to lock internal set of ports in use"))?;

        if ports.contains(&port) {
            return Err(anyhow!(
                "Cannot reserve port, port {} is already reserved",
                port
            ));
        }

        ports.insert(port);

        return Ok(Self { port });
    }

    pub fn reserve_random_port() -> Result<Self> {
        let mut ports = PORTS_IN_USE
            .lock()
            .map_err(|_| anyhow!("Failed to lock internal set of ports in use"))?;

        for _ in 0..MAX_TRIES {
            let port = pick_unused_port().context("No free port was found")?;
            ports.insert(port);

            return Ok(Self { port });
        }

        return Err(anyhow!(
            "Cannot find a free port, port finding exceeded the max number of tries"
        ));
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for ReservedPort {
    fn drop(&mut self) {
        PORTS_IN_USE
            .lock()
            .map(|mut ports| {
                ports.remove(&self.port);
            })
            .expect("Should be able to unlock reserved port on use");
    }
}
