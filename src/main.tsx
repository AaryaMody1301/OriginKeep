import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import AdoptExisting from "./AdoptExisting";
import OriginKeep2 from "./OriginKeep2";
import "./styles.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <OriginKeep2 />
    <AdoptExisting />
  </StrictMode>,
);
