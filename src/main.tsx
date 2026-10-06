import React from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import TrayPanel from "./TrayPanel";
import "./styles.css";
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {new URLSearchParams(location.search).has("tray") ? <TrayPanel /> : <App />}
  </React.StrictMode>,
);
