mod click;
mod click_match_template;
mod press;
mod swipe;
mod task;

use std::{fmt::Debug, time::Duration};

pub use click::*;
pub use click_match_template::*;
pub use press::*;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
pub use swipe::*;
pub use task::*;

use crate::{AutoPlay, HasController, resource::GetTemplate, task::GetTask};

#[skip_serializing_none]
#[derive(Clone, Debug, Serialize, Deserialize)]
/// TaskStep 是对 [`Action`] 的封装，可以设置一些额外的属性
pub struct ActionOptions {
    /// 在此 Step 开始前的延迟
    pub delay_sec: Option<f32>,
    /// 如果此 Step 失败，是否跳过（否则会直接中断退出）
    pub skip_if_failed: Option<bool>,
    /// 重复次数
    pub repeat: Option<u32>,
    /// 每次重试次数
    pub retry: Option<i32>,
}

/// Action are the tasks you can use in the configuration file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    /// [`Press`]
    Press(Press),
    /// [`Click`]
    Click(Click),
    /// [`Swipe`]
    Swipe(Swipe),
    /// [`ClickMatchTemplate`]
    ClickMatchTemplate(ClickMatchTemplate),
    Task(Task<Action>),
}

impl Action {
    pub fn press_esc() -> Self {
        Self::Press(Press::esc())
    }
    pub fn press_home() -> Self {
        Self::Press(Press::home())
    }
    pub fn click(x: u32, y: u32) -> Self {
        Self::Click(Click::new(x, y))
    }
    pub fn swipe(
        start: (u32, u32),
        end: (i32, i32),
        duration: Duration,
        slope_in: f32,
        slope_out: f32,
    ) -> Self {
        Self::Swipe(Swipe::new(start, end, duration, slope_in, slope_out))
    }
    pub fn click_match_template(template: impl AsRef<str>) -> Self {
        Self::ClickMatchTemplate(ClickMatchTemplate::new(template))
    }
    pub fn task(name: impl AsRef<str>) -> Self {
        Self::Task(Task::new(name))
    }
}

/// A [`Runnable`] can be executed with a reference of [`Runnable::Executor`],
/// and it will return a value of [`Runnable::Output`].
pub trait Runnable<T> {
    type Output;
    fn execute(&self, executor: &T) -> anyhow::Result<Self::Output>;
}

impl<T> Runnable<T> for Action
where
    T: HasController + GetTask<Action> + GetTemplate,
{
    type Output = ();
    fn execute(&self, executor: &T) -> anyhow::Result<Self::Output> {
        match self {
            Action::Press(action) => action.execute(executor),
            Action::Click(action) => action.execute(executor),
            Action::Swipe(action) => action.execute(executor),
            Action::ClickMatchTemplate(action) => action.execute(executor),
            Action::Task(action) => action.execute(executor),
        }
    }
}

/// The executor of an [`ActionExecutor::Action`].
///
/// Implementing this trait registers a specific [`Runnable<Executor = Self, Output = ()>`] type
/// to it self as the supported action.
pub trait ActionExecutor: Sized {
    type Action: Runnable<Self, Output = ()>;
    fn exec_action(&self, action: Self::Action) -> anyhow::Result<()> {
        action.execute(self)
    }
}

impl ActionExecutor for AutoPlay {
    type Action = Action;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde_action() {
        let action = Action::click(10, 10);
        let toml = toml::to_string_pretty(&action).unwrap();
        println!("{toml}");

        let action = Action::click_match_template("template.png");
        let toml = toml::to_string_pretty(&action).unwrap();
        println!("{toml}");

        let action = Action::swipe((10, 10), (20, 20), Duration::from_secs_f32(0.5), 0.1, 1.0);
        let toml = toml::to_string_pretty(&action).unwrap();
        println!("{toml}");

        let action = Action::press_esc();
        let toml = toml::to_string_pretty(&action).unwrap();
        println!("{toml}");

        let action = Action::task("test");
        let toml = toml::to_string_pretty(&action).unwrap();
        println!("{toml}");
    }
}
