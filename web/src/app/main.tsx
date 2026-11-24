import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { Providers } from "./providers";

import "./styles/index.css";

const rootElement = document.getElementById("root");

if (!rootElement) {
  throw new Error("Missing #root element");
}

if (import.meta.env.MODE === "mock") {
  const { startMockApi } = await import("./mock");
  await startMockApi();
}

createRoot(rootElement).render(
  <StrictMode>
    <Providers />
  </StrictMode>,
);
