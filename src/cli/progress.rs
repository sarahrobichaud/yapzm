//! Terminal progress indicators for long-running operations.

use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

use crate::store::CopyProgress;

/// How often indicators redraw while waiting on slow work.
const TICK_INTERVAL: Duration = Duration::from_millis(120);

/// Starts a spinner showing a message until it is cleared.
pub(crate) fn spinner(message: &str) -> ProgressBar {
    let bar = ProgressBar::new_spinner();
    bar.set_style(
        ProgressStyle::with_template("{spinner} {msg}").expect("the spinner template is valid"),
    );
    bar.set_message(message.to_owned());
    bar.enable_steady_tick(TICK_INTERVAL);
    bar
}

/// A progress bar for copying into the store, spinning until the copy size is known.
pub(crate) struct CopyBar {
    bar: ProgressBar,
}

impl CopyBar {
    /// Starts a spinner shown while the content to copy is being measured.
    pub(crate) fn new(measuring_message: &str) -> Self {
        Self {
            bar: spinner(measuring_message),
        }
    }
}

impl CopyProgress for CopyBar {
    /// Switches from the spinner to a byte-counting bar.
    fn started(&mut self, total_bytes: u64) {
        self.bar.set_length(total_bytes);
        self.bar.set_style(
            ProgressStyle::with_template(
                "{msg:>12} [{bar:30}] {binary_bytes}/{binary_total_bytes} \
                 {binary_bytes_per_sec} ETA {eta}",
            )
            .expect("the bar template is valid")
            .progress_chars("=> "),
        );
        self.bar.set_message("");
    }

    /// Names the content being copied.
    fn item_started(&mut self, label: &str) {
        self.bar.set_message(label.to_owned());
    }

    /// Moves the bar forward.
    fn advanced(&mut self, bytes: u64) {
        self.bar.inc(bytes);
    }

    /// Clears the bar so the summary prints cleanly.
    fn finished(&mut self) {
        self.bar.finish_and_clear();
    }
}
