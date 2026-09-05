import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import PassportCenter from "./PassportCenter";
import "./styles.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <PassportCenter />
    <App />
  </StrictMode>,
);
