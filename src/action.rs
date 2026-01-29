use std::time::Duration;

use ap_controller::ControllerTrait;
use serde::{Deserialize, Serialize};
use typetag;

#[typetag::serde]
pub trait Action {
    fn execute(&self, ap: &crate::AutoPlay) -> anyhow::Result<()>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Click {
    pub x: u32,
    pub y: u32,
}

#[typetag::serde]
impl Action for Click {
    fn execute(&self, ap: &crate::AutoPlay) -> anyhow::Result<()> {
        ap.click(self.x, self.y)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Key {
    Escape,
}

impl Into<ap_controller::Key> for Key {
    fn into(self) -> ap_controller::Key {
        match self {
            Key::Escape => ap_controller::Key::Escape,
        }
    }
}

impl Key {
    pub fn press(self) -> Press {
        Press { key: self }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Press {
    pub key: Key,
}

#[typetag::serde]
impl Action for Press {
    fn execute(&self, ap: &crate::AutoPlay) -> anyhow::Result<()> {
        ap.controller().press(self.key.into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Swipe {
    pub start: (u32, u32),
    pub end: (i32, i32),
    pub duration: Duration,
    pub slope_in: f32,
    pub slope_out: f32,
}

#[typetag::serde]
impl Action for Swipe {
    fn execute(&self, ap: &crate::AutoPlay) -> anyhow::Result<()> {
        ap.swipe(
            self.start,
            self.end,
            self.duration,
            self.slope_in,
            self.slope_out,
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WaitAction {
    pub ms: u64,
}

#[typetag::serde]
impl Action for WaitAction {
    fn execute(&self, _ap: &crate::AutoPlay) -> anyhow::Result<()> {
        std::thread::sleep(std::time::Duration::from_millis(self.ms));
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LaunchAppAction {
    pub package: String,
}

#[typetag::serde]
impl Action for LaunchAppAction {
    fn execute(&self, ap: &crate::AutoPlay) -> anyhow::Result<()> {
        use ap_controller::AndroidController;
        let android = ap
            .controller_ref::<AndroidController>()
            .ok_or_else(|| anyhow::anyhow!("not an android controller"))?;
        android.launch_app(&self.package)
    }
}

// Duration serialization module for TOML format (delay_sec = f32)
mod duration_secs_f32_option {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Option<Duration>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match duration {
            Some(d) => serializer.serialize_some(&d.as_secs_f32()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs: Option<f32> = Option::deserialize(deserializer)?;
        Ok(secs.map(Duration::from_secs_f32))
    }
}
