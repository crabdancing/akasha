#[macro_export]
macro_rules! printrn {
    () => {
        print!("\r\n")
    };
    ($($arg:tt)*) => {
        let start = format!($($arg)*);
        print!("{}\r\n", start);
    };
}