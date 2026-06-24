pub fn place() {
    reserve_inventory();
    super::shipping::dispatch();
}

pub fn reserve_inventory() {
    super::inventory::reserve();
}
