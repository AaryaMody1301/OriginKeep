import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import AdoptBar from "./AdoptBar";
import App from "./App";
import BrowserSetupBar from "./BrowserSetupBar";
import PassportCenter from "./PassportCenter";
import "./styles.css";
import "./passport.css";
import "./adopt.css";
import "./browser-setup.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <BrowserSetupBar />
    <AdoptBar />
    <PassportCenter />
    <App />
  </StrictMode>,
);
