pub fn reserve() {
    check_catalog();
}

pub fn check_catalog() {
    super::catalog::lookup();
}
