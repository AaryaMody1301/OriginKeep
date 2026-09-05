import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import AdoptFilePanel from "./AdoptFilePanel";
import App from "./App";
import PassportDashboard from "./PassportDashboard";
import "./styles.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <PassportDashboard />
    <AdoptFilePanel />
    <App />
  </StrictMode>,
);
