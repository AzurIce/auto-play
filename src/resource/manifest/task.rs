use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use std::fmt::Debug;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::{collections::HashMap, fs};

use crate::actions::Action;
use crate::task::{Task, TaskStep};

fn get_task_files(path: impl AsRef<Path>) -> Vec<PathBuf> {
    let mut task_files = vec![];
    if let Ok(read_dir) = fs::read_dir(path) {
        for entry in read_dir {
            let entry = entry.unwrap();
            let file_type = entry.file_type().unwrap();
            if file_type.is_dir() {
                task_files.extend(get_task_files(entry.path()));
            } else if file_type.is_file() {
                task_files.push(entry.path());
            }
        }
    }
    task_files
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TaskConfig<A: Debug + Clone>(pub HashMap<String, Task<A>>);

impl<A: Debug + Clone> Deref for TaskConfig<A> {
    type Target = HashMap<String, Task<A>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<A: Debug + Clone + DeserializeOwned> TaskConfig<A> {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, anyhow::Error> {
        let path = path.as_ref();
        let mut task_config = TaskConfig(HashMap::new());
        for task_file in get_task_files(path) {
            if let Ok(task) = fs::read_to_string(task_file) {
                let task = toml::from_str::<Task<A>>(&task)?;

                task_config.0.insert(task.name.to_string(), task);
            }
        }
        Ok(task_config)
    }
}

impl Default for TaskConfig<Action> {
    fn default() -> Self {
        let mut map = HashMap::new();
        let test_tasks = default_tasks();
        for task in test_tasks {
            map.insert(task.name.clone(), task);
        }
        Self(map)
    }
}

#[allow(unused)]
fn startup_task() -> Task<Action> {
    Task {
        name: "start_up".to_string(),
        desc: Some("start up to the main screen".to_string()),
        steps: vec![
            TaskStep::from_action(Action::click_match_template("start_start.png")).with_retry(-1),
            TaskStep::from_action(Action::click_match_template("wakeup_wakeup.png")).with_retry(-1),
            TaskStep::from_action(Action::click_match_template("confirm.png"))
                .with_delay(6.0)
                .with_retry(3)
                .skip_if_failed(),
            TaskStep::from_action(Action::click_match_template("qiandao_close.png"))
                .with_delay(2.0)
                .with_retry(2)
                .skip_if_failed(),
            TaskStep::from_action(Action::click_match_template("notice_close.png"))
                .with_delay(2.0)
                .with_retry(2)
                .skip_if_failed(),
        ],
    }
}

#[allow(unused)]
fn award_task() -> Task<Action> {
    Task {
        name: "award".to_string(),
        desc: None,
        steps: vec![
            TaskStep::from_action(Action::task("enter_mission")),
            TaskStep::from_action(Action::click_match_template("mission-week_collect-all.png"))
                .with_delay(0.5)
                .with_retry(1)
                .skip_if_failed(),
            TaskStep::from_action(Action::click_match_template("confirm.png"))
                .with_delay(0.5)
                .with_retry(1)
                .skip_if_failed(),
            TaskStep::from_action(Action::click_match_template("mission-day_week.png"))
                .with_delay(0.5)
                .with_retry(1),
            TaskStep::from_action(Action::click_match_template("mission-week_collect-all.png"))
                .with_delay(0.5)
                .with_retry(1)
                .skip_if_failed(),
            TaskStep::from_action(Action::click_match_template("confirm.png"))
                .with_delay(0.5)
                .with_retry(1)
                .skip_if_failed(),
            TaskStep::from_action(Action::task("back")),
        ],
    }
}

pub fn default_tasks() -> Vec<Task<Action>> {
    vec![
        Task {
            name: "press_esc".to_string(),
            desc: None,
            steps: vec![TaskStep::from_action(Action::press_esc())],
        },
        Task {
            name: "press_home".to_string(),
            desc: None,
            steps: vec![TaskStep::from_action(Action::press_home())],
        },
    ]
}

#[cfg(test)]
mod test {
    use std::{error::Error, fs::OpenOptions, io::Write};

    use super::*;

    #[test]
    fn test_task_config() {
        let defalt_config = TaskConfig::<Action>::default();
        println!("{:#?}", defalt_config);
        let toml = toml::to_string_pretty(&defalt_config).unwrap();
        println!("{toml}");

        let config = TaskConfig::<Action>::load("test/auto-play-resources");
        println!("{:#?}", config);
    }

    #[test]
    fn test_ser_task() {
        // let task = startup_task();
        let task = award_task();
        let config = toml::to_string_pretty(&task).unwrap();
        println!("{}", config);
    }

    #[test]
    fn write_default_task_config() -> Result<(), Box<dyn Error>> {
        let mut open_options = OpenOptions::new();
        open_options.write(true).create(true);
        let config = TaskConfig::default();
        let config_file = "../../resources/tasks.toml";

        {
            println!("{:?}", config);
            let config = toml::to_string_pretty(&config)?;
            println!("{}", config);
            let mut file = open_options.open(config_file)?;
            file.write_fmt(format_args!("{}", config))?;
        }

        // {
        //     let config = serde_json::to_string_pretty(&config)?;
        //     let config_file = "./resources/tasks.json";
        //     let mut file = open_options.open(config_file)?;
        //     file.write_fmt(format_args!("{}", config))?;
        // }

        // {
        //     let config = fs::read_to_string(config_file)?;
        //     let config: TaskConfig = toml::from_str(&config)?;
        //     println!("{:?}", config);
        // }
        Ok(())
    }
}
