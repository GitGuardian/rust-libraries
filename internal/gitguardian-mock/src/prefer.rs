pub const HEADER: &str = "prefer";

pub fn code(status: u16) -> String {
    format!("code={status}")
}

pub fn example(name: &str) -> String {
    format!("example={name}")
}
