use anyhow::Result;
use anyhow::anyhow;
use reserve_port::ReservedPort;

/// Returns a randomly selected port that is not in use.
pub fn new_random_port() -> Result<u16> {
    ReservedPort::random_permanently_reserved().map_err(|_| anyhow!("No free port was found"))
}
