//! Image matching based on template matching
//!
//! SingleMatcher: Match one template on an image to get one result.
//! MultiMatcher: Match one template on an image to get multiple results.
//! BestMatcher: Match one template on many images to get the best one.

pub struct MatchOptions {
    template: image::DynamicImage,
}

pub struct SingleMatcher {}

