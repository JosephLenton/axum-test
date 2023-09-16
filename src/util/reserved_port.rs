use ::anyhow::anyhow;
use ::anyhow::Context;
use ::anyhow::Result;
use ::lazy_static::lazy_static;
use ::portpicker::pick_unused_port;
use ::std::collections::HashMap;
use ::std::sync::Mutex;
use ::std::sync::Arc;

const MAX_TRIES: u32 = 10;

lazy_static! {
    static ref PORTS_IN_USE: Mutex<HashMap<u16, usize>> = Mutex::new(HashMap::new());
}

#[derive(Debug)]
pub struct ReservedPort {
    port: u16,
    count: usize,
}

impl ReservedPort {
    #[must_use]
    pub fn add_port_reservation(port: u16) -> Result<Self> {
        let mut ports = PORTS_IN_USE
            .lock()
            .map_err(|_| anyhow!("Failed to lock internal set of ports in use"))?;

        let count = ports.get(&port).cloned().unwrap_or(0);
        ports.insert(port, count+1);

        let reserved_port = Self {
            port,
            count,
        };
        return Ok(reserved_port);
    }

    #[must_use]
    pub fn reserve_random_port() -> Result<Self> {
        let mut ports = PORTS_IN_USE
            .lock()
            .map_err(|_| anyhow!("Failed to lock internal set of ports in use"))?;

        for _ in 0..MAX_TRIES {
            let port = pick_unused_port().context("No free port was found")?;

            if !ports.contains_key(&port) {
                let reservation_flag = Arc::new(());
                ports.insert(port, 1);

                let reserved_port = Self {
                    port,
                    count: 1,
                };
                return Ok(reserved_port);
            }
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
        let port = self.port;

        PORTS_IN_USE
            .lock()
            .map(|mut ports| {
                if let Some(port_count) = ports.get(&self.port).cloned() {
                    if port_count > 1 {
                        ports.insert(self.port, port_count - 1);
                    } else {
                        ports.remove(&self.port);
                    }
                }
            })
            .expect("Should be able to unlock reserved port on use");
    }
}

#[cfg(test)]
mod test_reserve_port {
    use super::*;

    #[test]
    fn it_should_reserve_a_port_for_use() {
        const TEST_PORT_NUM: u16 = 1230;

        let reserved = ReservedPort::add_port_reservation(TEST_PORT_NUM).unwrap();

        assert_eq!(reserved.port(), TEST_PORT_NUM);
    }

    #[test]
    fn it_should_reserve_same_port_twice_in_a_row() {
        const TEST_PORT_NUM: u16 = 1231;

        let _reserved = ReservedPort::add_port_reservation(TEST_PORT_NUM).unwrap();
        let reserved_two = ReservedPort::add_port_reservation(TEST_PORT_NUM);

        assert!(reserved_two.is_ok());
    }

    #[test]
    fn it_should_allow_reserving_ports_after_dropped() {
        const TEST_PORT_NUM: u16 = 1232;

        let reserved = ReservedPort::add_port_reservation(TEST_PORT_NUM).unwrap();
        std::mem::drop(reserved);

        let reserved_two = ReservedPort::add_port_reservation(TEST_PORT_NUM).unwrap();

        assert_eq!(reserved_two.port(), TEST_PORT_NUM);
    }

    #[test]
    fn it_should_allow_reserving_random_ports_by_hand() {
        let reserved_1 = ReservedPort::reserve_random_port().unwrap();
        let reserved_2 = ReservedPort::add_port_reservation(reserved_1.port());

        assert!(reserved_2.is_ok());
    }

    #[test]
    fn it_should_allow_reserving_random_ports_by_hand_after_they_have_dropped() {
        let reserved_1 = ReservedPort::reserve_random_port().unwrap();
        let random_port = reserved_1.port();
        ::std::mem::drop(reserved_1);

        let reserved_2 = ReservedPort::add_port_reservation(random_port).unwrap();

        assert_eq!(reserved_2.port(), random_port);
    }
}

#[cfg(test)]
mod test_reserve_random_port {
    use super::*;

    #[test]
    fn it_should_reserve_a_random_port_for_use() {
        let reserved = ReservedPort::reserve_random_port();

        assert!(reserved.is_ok());
    }

    #[test]
    fn it_should_reserve_different_ports_over_use() {
        let reserved_1 = ReservedPort::reserve_random_port().unwrap();
        let reserved_2 = ReservedPort::reserve_random_port().unwrap();

        assert_ne!(reserved_1.port(), reserved_2.port());
    }
}
