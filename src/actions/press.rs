use serde::{Deserialize, Serialize};

use crate::{HasController, actions::Runnable};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Key {
    Esc,
    Home,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Press {
    key: Key,
}

impl Press {
    pub fn esc() -> Self {
        Self { key: Key::Esc }
    }
    pub fn home() -> Self {
        Self { key: Key::Home }
    }
}

impl<T: HasController> Runnable<T> for Press {
    type Output = ();
    fn execute(&self, executor: &T) -> anyhow::Result<Self::Output> {
        match self.key {
            Key::Esc => executor.controller().press_esc(),
            Key::Home => executor.controller().press_home(),
        }
        .map_err(|err| anyhow::anyhow!("controller error: {:?}", err))?;
        Ok(())
    }
}
