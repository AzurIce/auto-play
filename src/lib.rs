pub mod actions;
pub mod resource;
pub mod task;

use std::path::Path;

use ap_controller::Controller;
use image::DynamicImage;

use crate::{
    actions::Action,
    resource::{GetTemplate, Resource},
    task::{GetTask, Task},
};

pub trait HasController {
    fn controller(&self) -> &Controller;
}

pub struct AutoPlay {
    controller: Controller,
    resource: Resource<Action>,
}

impl HasController for AutoPlay {
    fn controller(&self) -> &Controller {
        &self.controller
    }
}

impl GetTask<Action> for AutoPlay {
    fn get_task(&self, name: impl AsRef<str>) -> Option<&Task<Action>> {
        self.resource.get_task(name)
    }
}

impl GetTemplate for AutoPlay {
    fn get_template(&self, path: impl AsRef<Path>) -> anyhow::Result<DynamicImage> {
        self.resource.get_template(path)
    }
}

impl AutoPlay {
    pub fn new(controller: Controller, resource: Resource<Action>) -> Self {
        Self {
            controller,
            resource,
        }
    }
}
