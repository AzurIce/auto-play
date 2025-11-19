use std::fmt::Debug;

use serde::{Deserialize, Serialize};

use crate::{HasController, actions::Runnable, task::GetTask};

/// An action for clicking the specific coordinate on the screen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task<A: Debug + Clone> {
    name: String,
    #[serde(skip)]
    _phantom: std::marker::PhantomData<A>,
}

impl<A: Debug + Clone> Task<A> {
    pub fn new(name: impl AsRef<str>) -> Self {
        Self {
            name: name.as_ref().to_string(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T, A> Runnable<T> for Task<A>
where
    T: HasController + GetTask<A>,
    A: Debug + Clone + Runnable<T, Output = ()>,
{
    type Output = ();
    fn execute(&self, executor: &T) -> anyhow::Result<Self::Output> {
        let task = executor.get_task(&self.name).ok_or(anyhow::anyhow!(
            "failed to get task by name: {:?}",
            self.name
        ))?;
        task.execute(executor).map(|_| ())
    }
}
