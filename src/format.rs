/// Print a note message with a newline
macro_rules! note {
    ($msg:expr) => {
        println!("{}: {}", "note".bold().cyan(), $msg);
    };
}
pub(crate) use note;

/// Print a tip message with a newline
macro_rules! tip {
    ($msg:expr) => {
        println!("  {}: {}", "tip".bold().cyan(), $msg);
    };
}
pub(crate) use tip;

/// Print an error message with a newline
macro_rules! error {
    ($msg:expr) => {
        eprintln!("{}: {}", "error".bold().red(), $msg);
    };
}
pub(crate) use error;

/// Pluralize a word based on a count
macro_rules! pluralize {
    ($word:expr, $count:expr) => {
        if $count == 1 {
            $word.to_string()
        } else {
            format!("{}s", $word)
        }
    };
}
pub(crate) use pluralize;
