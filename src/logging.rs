// flycheck-rust has issues with the log crate being in two different
// places (required here and in the compiler itself). Redefine these
// macros as println!s to make flycheck happy for now.
macro_rules! error { ($($arg:tt)*) => (println!($($arg)+);) }
macro_rules! warn  { ($($arg:tt)*) => (println!($($arg)+);) }
macro_rules! info  { ($($arg:tt)*) => (println!($($arg)+);) }
macro_rules! debug { ($($arg:tt)*) => (println!($($arg)+);) }
