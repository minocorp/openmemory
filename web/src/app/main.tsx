import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { Providers } from "./providers";

import "./styles/index.css";

const rootElement = document.getElementById("root");

if (!rootElement) {
  throw new Error("Missing #root element");
}

createRoot(rootElement).render(
  <StrictMode>
    <Providers />
  </StrictMode>,
);
