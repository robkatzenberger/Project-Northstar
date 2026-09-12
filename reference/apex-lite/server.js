const { createApp } = require("./src/server");

const port = Number(process.env.PORT || 3000);
const server = createApp();

server.listen(port, () => {
  console.log(`APEX-Lite console listening on http://localhost:${port}`);
});
