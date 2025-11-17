//! An [`App`] is an application that will be pushed to the device and run.

// pub mod minicap;
pub mod maatouch;

use ap_adb::Device;

pub trait App {
    fn check(device: &Device) -> anyhow::Result<()>;
    fn push(device: &Device) -> anyhow::Result<()>;
    fn prepare(device: &Device) -> anyhow::Result<()> {
        if Self::check(device).is_err() {
            Self::push(device)?;
            Self::check(device)?;
        }
        Ok(())
    }

    fn init(device: &Device) -> anyhow::Result<Self>
    where
        Self: Sized;
}