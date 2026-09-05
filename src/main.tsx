import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import AdoptBar from "./AdoptBar";
import App from "./App";
import PassportCenter from "./PassportCenter";
import "./styles.css";
import "./passport.css";
import "./adopt.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <AdoptBar />
    <PassportCenter />
    <App />
  </StrictMode>,
);
