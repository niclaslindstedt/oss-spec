// §11.3.1 — SSR entry. `prerender.mjs` calls `render(url)` at build time
// to produce the static HTML body that gets spliced into `dist/index.html`.
// The client entry (`main.tsx`) uses `hydrateRoot` to attach over the
// rendered tree without wiping it.

import { StrictMode } from "react";
import { renderToString } from "react-dom/server";
import { StaticRouter } from "react-router-dom";
import App from "./App";

export function render(url: string): string {
  return renderToString(
    <StrictMode>
      <StaticRouter location={url} basename="/oss-spec/">
        <App />
      </StaticRouter>
    </StrictMode>,
  );
}
