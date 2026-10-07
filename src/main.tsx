import React from "react";
import { Sentry, initializeDiagnostics } from "./observability";
import { createRoot } from "react-dom/client";
import App from "./App";
import TrayPanel from "./TrayPanel";
import "./styles.css";
void initializeDiagnostics().finally(() => {
  createRoot(document.getElementById("root")!, {
    onUncaughtError: Sentry.reactErrorHandler(),
    onRecoverableError: Sentry.reactErrorHandler(),
  }).render(
    <React.StrictMode>
      <Sentry.ErrorBoundary
        beforeCapture={(scope) =>
          scope.setTag("maxxit_event", "ui.render.failed")
        }
        fallback={
          <div role="alert">
            Maxxit couldn't display this window. Quit and reopen the app.
          </div>
        }
      >
        {new URLSearchParams(location.search).has("tray") ? (
          <TrayPanel />
        ) : (
          <App />
        )}
      </Sentry.ErrorBoundary>
    </React.StrictMode>,
  );
});
