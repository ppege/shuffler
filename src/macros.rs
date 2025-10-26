
#[macro_export]
macro_rules! spinner {
    ($fn:expr, $message:expr, $finish_message:expr) => {
        async {
            use indicatif::{ProgressBar, ProgressStyle};
            use std::time::Duration;
            let spinner_style = ProgressStyle::with_template("{msg} {spinner}").unwrap();
            let tick_duration = Duration::from_millis(100);
            let spinner = ProgressBar::new_spinner()
                .with_style(spinner_style.clone())
                .with_message($message);
            spinner.enable_steady_tick(tick_duration);
            let result = $fn.await;
            spinner.finish_with_message(format!("[\u{2713}] {}", $finish_message));
            result
        }
    };
}
