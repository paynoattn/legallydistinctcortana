use std::sync::{Mutex, PoisonError};

/// Numbers the readings so only the latest one reports how it ended. A new reading is
/// announced, and an old one reported, while the number is held, so a reading stopped by a
/// voice server change cannot overwrite the status of one started since.
#[derive(Debug, Default)]
pub struct Readings(Mutex<u64>);

impl Readings {
    /// Numbers a new reading and runs `announce` before any older reading can report.
    pub fn begin(&self, announce: impl FnOnce()) -> u64 {
        let mut latest = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        *latest = latest.wrapping_add(1);
        announce();
        *latest
    }

    /// Runs `report` if `reading` is still the latest one; `None` if a newer one began.
    pub fn report_if_latest<T>(&self, reading: u64, report: impl FnOnce() -> T) -> Option<T> {
        let latest = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        (*latest == reading).then(report)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::mpsc;
    use std::thread;

    use super::*;

    #[test]
    fn each_reading_gets_the_next_number_and_is_announced() {
        let readings = Readings::default();
        let mut announced = 0;
        let first = readings.begin(|| announced += 1);
        let second = readings.begin(|| announced += 1);
        assert_eq!((first, second, announced), (1, 2, 2));
    }

    #[test]
    fn only_the_latest_reading_reports() {
        let readings = Readings::default();
        let first = readings.begin(|| ());
        assert_eq!(readings.report_if_latest(first, || "first"), Some("first"));
        let second = readings.begin(|| ());
        assert_eq!(readings.report_if_latest(first, || "first"), None);
        assert_eq!(
            readings.report_if_latest(second, || "second"),
            Some("second")
        );
    }

    #[test]
    fn a_new_reading_waits_for_a_report_in_progress() {
        let readings = Arc::new(Readings::default());
        let first = readings.begin(|| ());
        let (in_report, report_entered) = mpsc::channel();
        let (finish, finished) = mpsc::channel::<()>();
        let reporter = {
            let readings = Arc::clone(&readings);
            thread::spawn(move || {
                readings.report_if_latest(first, || {
                    in_report.send(()).unwrap();
                    finished.recv().unwrap();
                })
            })
        };
        report_entered.recv().unwrap();
        let (numbered, numbers) = mpsc::channel();
        let newer = {
            let readings = Arc::clone(&readings);
            thread::spawn(move || numbered.send(readings.begin(|| ())).unwrap())
        };
        assert!(
            numbers
                .recv_timeout(std::time::Duration::from_millis(50))
                .is_err(),
            "the new reading waits for the report"
        );
        finish.send(()).unwrap();
        assert_eq!(reporter.join().unwrap(), Some(()));
        newer.join().unwrap();
        assert_eq!(numbers.recv().unwrap(), 2);
    }
}
