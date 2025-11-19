//! Image matching based on template matching
//!
//! [`SingleMatcher`]: Match one template on an image to get one result.
//! [`MultiMatcher`]: Match one template on an image to get multiple results.
//! [`BestMatcher`]: Match one template on many images to get the best one.

use image::{ImageBuffer, Luma, math::Rect};
use imageproc::template_matching::find_extremes;

use crate::core::template_matching::{Match, MatchTemplateMethod, find_matches, match_template};

pub struct MatcherOptions {
    pub method: MatchTemplateMethod,
    pub threshold: f32,
    pub padding: bool,
}

impl Default for MatcherOptions {
    fn default() -> Self {
        Self {
            method: MatchTemplateMethod::SumOfSquaredDifferenceNormed,
            threshold: 0.2,
            padding: false,
        }
    }
}

impl MatcherOptions {
    pub fn method_default(method: MatchTemplateMethod) -> Self {
        let mut options = Self::default();
        options.method = method;
        options.threshold = match method {
            MatchTemplateMethod::SumOfSquaredDifference
            | MatchTemplateMethod::CrossCorrelation
            | MatchTemplateMethod::CorrelationCoefficient => 30.0,
            MatchTemplateMethod::SumOfSquaredDifferenceNormed => 0.2,
            MatchTemplateMethod::CrossCorrelationNormed
            | MatchTemplateMethod::CorrelationCoefficientNormed => 0.8,
        };
        options
    }
    pub fn with_method(mut self, method: MatchTemplateMethod) -> Self {
        self.method = method;
        self
    }
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold;
        self
    }
    pub fn padded(mut self) -> Self {
        self.padding = true;
        self
    }
}

/// Match one template on an image to get one result.
pub struct SingleMatcher;

pub struct SingleMatcherResult {
    pub result: Option<Match>,
    pub matched_image: ImageBuffer<Luma<f32>, Vec<f32>>,
}

impl SingleMatcher {
    pub fn match_template(
        image: &ImageBuffer<Luma<f32>, Vec<f32>>,
        template: &ImageBuffer<Luma<f32>, Vec<f32>>,
        options: &MatcherOptions,
    ) -> SingleMatcherResult {
        use MatchTemplateMethod::*;

        let matched_image = match_template(image, template, options.method, options.padding);
        let extremes = find_extremes(&matched_image);
        let result = match options.method {
            SumOfSquaredDifference | SumOfSquaredDifferenceNormed => {
                if extremes.min_value < options.threshold {
                    Some(Match {
                        rect: Rect {
                            x: extremes.min_value_location.0,
                            y: extremes.min_value_location.1,
                            width: template.width(),
                            height: template.height(),
                        },
                        value: extremes.min_value,
                    })
                } else {
                    None
                }
            }
            CrossCorrelation
            | CrossCorrelationNormed
            | CorrelationCoefficient
            | CorrelationCoefficientNormed => {
                if extremes.max_value > options.threshold {
                    Some(Match {
                        rect: Rect {
                            x: extremes.max_value_location.0,
                            y: extremes.max_value_location.1,
                            width: template.width(),
                            height: template.height(),
                        },
                        value: extremes.max_value,
                    })
                } else {
                    None
                }
            }
        };
        SingleMatcherResult {
            result,
            matched_image,
        }
    }
}

/// Match one template on an image to get multiple results.
pub struct MultiMatcher;

pub struct MultiMatcherResult {
    pub result: Vec<Match>,
    pub matched_image: ImageBuffer<Luma<f32>, Vec<f32>>,
}

impl MultiMatcher {
    pub fn match_template(
        image: &ImageBuffer<Luma<f32>, Vec<f32>>,
        template: &ImageBuffer<Luma<f32>, Vec<f32>>,
        options: &MatcherOptions,
    ) -> MultiMatcherResult {
        use MatchTemplateMethod::*;

        let matched_image = match_template(image, template, options.method, options.padding);

        let result = find_matches(
            &matched_image,
            template.width(),
            template.height(),
            options.method,
            options.threshold,
        )
        .into_iter()
        .filter(|m| match options.method {
            SumOfSquaredDifference | SumOfSquaredDifferenceNormed => m.value < options.threshold,
            CrossCorrelation
            | CrossCorrelationNormed
            | CorrelationCoefficient
            | CorrelationCoefficientNormed => m.value > options.threshold,
        })
        .collect();

        MultiMatcherResult {
            result,
            matched_image,
        }
    }
}

pub struct BestMatcher;

pub struct BestMatcherResult {
    pub result: Option<(usize, Match)>,
    pub single_results: Vec<SingleMatcherResult>,
}

impl BestMatcher {
    pub fn match_template<'a, I>(
        images: I,
        template: &ImageBuffer<Luma<f32>, Vec<f32>>,
        options: &MatcherOptions,
    ) -> BestMatcherResult
    where
        I: IntoIterator<Item = &'a ImageBuffer<Luma<f32>, Vec<f32>>>,
    {
        let single_results = images
            .into_iter()
            .map(|img| SingleMatcher::match_template(img, template, options))
            .collect::<Vec<_>>();

        let result = single_results
            .iter()
            .enumerate()
            .filter_map(|(i, res)| res.result.as_ref().map(|m| (i, *m)))
            .max_by(|(_, a), (_, b)| a.value.total_cmp(&b.value));

        BestMatcherResult {
            result,
            single_results,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_matcher() {
        let template = image::open("./assets/battle_deploy-card-cost1.png")
            .unwrap()
            .to_luma32f();
        let image = image::open("./assets/in_battle.png").unwrap().to_luma32f();

        for method in MatchTemplateMethod::ALL {
            let res = SingleMatcher::match_template(
                &image,
                &template,
                &MatcherOptions::method_default(method),
            );
            println!("Single: {method} - {:?}", res.result);
            if matches!(
                method,
                MatchTemplateMethod::SumOfSquaredDifference
                    | MatchTemplateMethod::CrossCorrelation
                    | MatchTemplateMethod::CorrelationCoefficient
            ) {
                continue;
            }
            let res = MultiMatcher::match_template(
                &image,
                &template,
                &MatcherOptions::method_default(method),
            );
            println!("Multi({}): {method} - {:?}", res.result.len(), res.result);
        }
    }

    #[test]
    fn test_best_matcher() {
        let images = [
            image::open("./assets/avatars/amiya.png")
                .unwrap()
                .to_luma32f(),
            image::open("./assets/avatars/angel_sale#8.png")
                .unwrap()
                .to_luma32f(),
            image::open("./assets/avatars/kalts.png")
                .unwrap()
                .to_luma32f(),
        ];
        let template = image::open("./assets/avatars/amiya.png")
            .unwrap()
            .to_luma32f();
        for method in MatchTemplateMethod::ALL {
            let res = BestMatcher::match_template(
                &images,
                &template,
                &MatcherOptions::method_default(method),
            );
            println!("Best: {method} - {:?}", res.result);
        }
    }
}
