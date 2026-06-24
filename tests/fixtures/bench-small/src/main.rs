mod services;

fn main() {
    services::billing::run_billing();
    services::users::run_users();
}
