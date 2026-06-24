// Stub for fixture parsing (no node_modules required).
function express(): {
  get(path: string, handler: () => unknown): void;
  post(path: string, handler: () => unknown): void;
} {
  return { get: () => {}, post: () => {} };
}

function listUsers() {
  return [];
}

const app = express();
app.get("/users", listUsers);
app.post("/users", createUser);

function createUser() {
  return {};
}
