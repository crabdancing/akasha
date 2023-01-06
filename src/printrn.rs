#[macro_export]
macro_rules! printrn {
    () => {
        print!("\r\n")
    };
    ($($arg:tt)*) => {
        let start = format!($($arg)*).replace("\n", "\r\n");
        print!("{}\r\n", start);
    };
}