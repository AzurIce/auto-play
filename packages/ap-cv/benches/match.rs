use ap_cv::{
    core::template_matching::{MatchTemplateMethod, find_matches, match_template},
    matcher::MatcherOptions,
};
use criterion::{Criterion, criterion_group, criterion_main};
use imageproc::template_matching::find_extremes;

fn bench_template_matching(c: &mut Criterion) {
    let image = image::open("./assets/in_battle.png").unwrap().to_luma32f();
    let template = image::open("./assets/battle_deploy-card-cost1.png")
        .unwrap()
        .to_luma32f();

    {
        let mut group = c.benchmark_group("match_template");
        for method in MatchTemplateMethod::ALL {
            group.bench_function(method.to_string(), |b| {
                b.iter(|| {
                    match_template(&image, &template, method, false);
                });
            });
        }
    }

    {
        let mut group = c.benchmark_group("find_extremes");
        for method in MatchTemplateMethod::ALL {
            let res = match_template(&image, &template, method, false);
            group.bench_function(method.to_string(), |b| {
                b.iter(|| find_extremes(&res));
            });
        }
    }

    {
        let mut group = c.benchmark_group("find_matches");
        for method in MatchTemplateMethod::ALL {
            let res = match_template(&image, &template, method, false);
            let options = MatcherOptions::method_default(method).padded();
            group.bench_function(method.to_string(), |b| {
                b.iter(|| {
                    find_matches(
                        &res,
                        template.width(),
                        template.height(),
                        method,
                        options.threshold,
                    )
                });
            });
        }
    }
}

criterion_group!(benches, bench_template_matching);
criterion_main!(benches);
