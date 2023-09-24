//! Anchor adapter

use anchor_client::solana_sdk::sysvar::clock::Clock;
use anchor_lang::prelude::*;
use bincode::deserialize;
use std::io::Write;
use std::ops::Deref;
/// Anchor wrapper for Clock
#[derive(Clone)]
pub struct AClock(Clock);

impl anchor_lang::AccountDeserialize for AClock {
    fn try_deserialize(buf: &mut &[u8]) -> Result<Self> {
        AClock::try_deserialize_unchecked(buf)
    }

    fn try_deserialize_unchecked(buf: &mut &[u8]) -> Result<Self> {
        let clock = deserialize::<Clock>(&buf);
        if clock.is_err() {
            return Err(error!(CustomError::ClockError));
        }
        Ok(AClock(clock.unwrap()))
    }
}

impl anchor_lang::AccountSerialize for AClock {
    fn try_serialize<W: Write>(&self, _writer: &mut W) -> Result<()> {
        // no-op
        Ok(())
    }
}

impl anchor_lang::Owner for AClock {
    fn owner() -> Pubkey {
        Pubkey::default()
    }
}

impl Deref for AClock {
    type Target = Clock;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[error_code]
pub enum CustomError {
    #[msg("Cannot decode clock")]
    ClockError,
}
