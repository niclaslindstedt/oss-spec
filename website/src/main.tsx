import { StrictMode } from "react";
import { hydrateRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import App from "./App";
import "./App.css";

// §11.3.1.3 — hydrate the prerendered tree emitted by `entry-server.tsx`.
// `createRoot` would wipe the SSR body, defeating the indexing-killer
// guard in §11.3.1.
hydrateRoot(
  document.getElementById("root")!,
  <StrictMode>
    <BrowserRouter basename="/oss-spec/">
      <App />
    </BrowserRouter>
  </StrictMode>,
);
