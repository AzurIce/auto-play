use std::fmt::Debug;

use ap_cv::matcher::{MatcherOptions, SingleMatcher};
use serde::{Deserialize, Serialize};

use crate::{HasController, actions::Runnable, resource::GetTemplate};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickMatchTemplate {
    template: String,
}

impl ClickMatchTemplate {
    pub fn new(template: impl AsRef<str>) -> Self {
        Self {
            template: template.as_ref().to_string(),
        }
    }
}

impl<T: HasController + GetTemplate> Runnable<T> for ClickMatchTemplate {
    type Output = ();
    fn execute(&self, executor: &T) -> anyhow::Result<Self::Output> {
        let template = executor.get_template(&self.template)?.to_luma32f();
        let screen = executor.controller().screencap()?.to_luma32f();

        let res = SingleMatcher::match_template(&screen, &template, &MatcherOptions::default());

        let rect = res
            .result
            .map(|m| m.rect)
            .ok_or(anyhow::anyhow!("failed to match {}", self.template))?;
        executor
            .controller()
            .click_in_rect(rect)
            .map_err(|err| anyhow::anyhow!("controller error: {:?}", err))?;
        Ok(())
    }
}
