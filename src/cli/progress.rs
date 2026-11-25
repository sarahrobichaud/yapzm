//! Terminal progress bars for long-running operations.

use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

use crate::store::CopyProgress;
use crate::zomboid::WorkshopItem;

/// How often the bar redraws while waiting on slow work.
const TICK_INTERVAL: Duration = Duration::from_millis(120);

/// A progress bar for copying workshop items, spinning until the copy size is known.
pub(crate) struct CopyBar {
    bar: ProgressBar,
}

impl CopyBar {
    /// Starts a spinner shown while the copy is being measured.
    pub(crate) fn new() -> Self {
        let bar = ProgressBar::new_spinner();
        bar.set_style(
            ProgressStyle::with_template("{spinner} {msg}").expect("the spinner template is valid"),
        );
        bar.set_message("Measuring workshop items...");
        bar.enable_steady_tick(TICK_INTERVAL);
        Self { bar }
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

    /// Names the item being copied.
    fn item_started(&mut self, item: &WorkshopItem) {
        self.bar.set_message(item.id.to_string());
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
