//! Holding and copying several styles in one function fits a small stack.
//! Its own test binary: an overflow aborts the process, not just one test.

use shadcn_egui::Style;

/// wasm and Windows main threads have 1 MiB of stack.
#[test]
fn seven_styles_cloned_three_times_fit_one_mib() {
    let names = std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(|| {
            let styles = [
                Style::vega(),
                Style::nova(),
                Style::maia(),
                Style::lyra(),
                Style::mira(),
                Style::luma(),
                Style::rhea(),
            ];
            let mut names = Vec::new();
            for s in &styles {
                let (a, b, c) = (s.clone(), s.clone(), s.clone());
                assert_eq!(a, b);
                assert_eq!(b, c);
                names.push(std::hint::black_box(c).name);
            }
            names
        })
        .expect("spawn")
        .join()
        .expect("thread");
    assert_eq!(
        names,
        ["vega", "nova", "maia", "lyra", "mira", "luma", "rhea"]
    );
}
