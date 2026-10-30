//! Strict fail-closed assertion macros for opaque-box E2E tests.
//!
//! Remediation of Codex Defect F31: Test runners must exit with a non-zero code
//! on failure rather than printing "FAIL" and returning exit code 0.

/// Assert condition is true; abort with exit code 1 if false.
#[macro_export]
macro_rules! fail_closed_assert {
    ($cond:expr, $($arg:tt)+) => {
        if !($cond) {
            eprintln!("\n========================================================");
            eprintln!("FAIL-CLOSED ASSERTION FAILED in {}:{}", file!(), line!());
            eprintln!("Condition: {}", stringify!($cond));
            eprintln!("Message:   {}", format_args!($($arg)+));
            eprintln!("========================================================\n");
            std::process::exit(1);
        }
    };
    ($cond:expr) => {
        fail_closed_assert!($cond, "Assertion condition evaluated to false");
    };
}

/// Assert left equals right; abort with exit code 1 if unequal.
#[macro_export]
macro_rules! fail_closed_assert_eq {
    ($left:expr, $right:expr, $($arg:tt)+) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(left_val == right_val) {
                    eprintln!("\n========================================================");
                    eprintln!("FAIL-CLOSED EQUALITY ASSERTION FAILED in {}:{}", file!(), line!());
                    eprintln!("Left:     {:?}", left_val);
                    eprintln!("Right:    {:?}", right_val);
                    eprintln!("Message:  {}", format_args!($($arg)+));
                    eprintln!("========================================================\n");
                    std::process::exit(1);
                }
            }
        }
    };
    ($left:expr, $right:expr) => {
        fail_closed_assert_eq!($left, $right, "Values are not equal");
    };
}

/// Driver for executing assertions with strict fail-closed exit semantics and progress tracking.
///
/// Remediates Codex Defect F31: Test drivers must record progress, track check counts,
/// flush streams, and exit non-zero immediately upon assertion failure.
#[derive(Debug, Clone)]
pub struct FailClosedDriver {
    /// Identifier for the driver instance or test fixture.
    pub name: String,
    /// Total count of checks executed.
    pub checks_run: usize,
    /// Ordered list of step names completed.
    pub steps_completed: Vec<String>,
    /// Step log messages recorded during execution.
    pub step_logs: Vec<String>,
}

impl FailClosedDriver {
    /// Initialize a new driver instance.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            checks_run: 0,
            steps_completed: Vec::new(),
            step_logs: Vec::new(),
        }
    }

    /// Record entry and completion of a named step in a multi-step fixture.
    pub fn record_step(&mut self, step_name: &str) {
        self.steps_completed.push(step_name.to_string());
        let log_msg = format!(
            "[{}] Step {}: {} - SUCCESS",
            self.name,
            self.steps_completed.len(),
            step_name
        );
        eprintln!("{log_msg}");
        self.step_logs.push(log_msg);
    }

    /// Execute a boolean check with strict fail-closed exit on failure.
    pub fn check(&mut self, description: &str, condition: bool) {
        self.checks_run += 1;
        if !condition {
            use std::io::Write;
            eprintln!("\n========================================================");
            eprintln!("[FAIL-CLOSED DRIVER] Assertion failed in '{}'", self.name);
            eprintln!("Check #{}: {}", self.checks_run, description);
            eprintln!("========================================================\n");
            let _ = std::io::stdout().flush();
            let _ = std::io::stderr().flush();
            std::process::exit(1);
        }
    }

    /// Flush output streams and return clean exit code 0.
    pub fn finish(&self) -> i32 {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        0
    }
}
