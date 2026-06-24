// Stub for fixture parsing (no node_modules required).
const axios = {
  get(path: string): void {},
};

export function fetchUsers() {
  axios.get("/users");
}
