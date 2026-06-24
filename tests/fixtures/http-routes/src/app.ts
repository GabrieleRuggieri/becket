function listUsers() {
  return [];
}

const app = express();
app.get("/users", listUsers);
app.post("/users", createUser);

function createUser() {
  return {};
}
