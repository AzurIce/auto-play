use serde::{Deserialize, Serialize};

use crate::{HasController, actions::Runnable};

/// An action for clicking the specific coordinate on the screen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Click {
    x: u32,
    y: u32,
}

impl Click {
    pub fn new(x: u32, y: u32) -> Self {
        Self { x, y }
    }
}

impl<T: HasController> Runnable<T> for Click {
    type Output = ();
    fn execute(&self, executor: &T) -> anyhow::Result<Self::Output> {
        executor
            .controller()
            .click(self.x, self.y)
            .map_err(|err| anyhow::anyhow!("controller error: {:?}", err))
    }
}
