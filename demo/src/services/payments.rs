pub fn capture() {
    super::orders::place();
}

pub fn authorize() {
    capture();
}
