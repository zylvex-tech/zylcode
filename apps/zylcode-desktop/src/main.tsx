import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/themes.css";
import "./index.css";
import "@xterm/xterm/css/xterm.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);