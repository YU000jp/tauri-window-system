import { createDeferred, lazy, onMount, Show, Suspense } from "solid-js"
import { WindowFrame } from "tauri-window-ui"
import { scheduleIdleWork } from "./bootTelemetry"
import { ControlBar, FooterPanel, StatusPanel } from "./hostPanels"
import { ShellFrame } from "./shellFrame"
import { useWindowSystem } from "./windowSystem"

const LazyRegistryPanel = lazy(() => import("./registryPanel"));
const LazyBusPanel = lazy(() => import("./busPanel"));
// Default to the lighter local shell so the fastest path is the normal path.
const USE_MINIMAL_SHELL = import.meta.env.VITE_SOLID_HOST_MINIMAL_SHELL !== "0";

export default function App() {
  const windowSystem = useWindowSystem();
  const isRootWindow = windowSystem.isRootWindow();
  const statusSummary = createDeferred(windowSystem.statusSummary, { timeoutMs: 120 });
  const Frame = USE_MINIMAL_SHELL ? ShellFrame : WindowFrame;

  onMount(() => {
    if (!isRootWindow) {
      return;
    }

    scheduleIdleWork(() => {
      void LazyRegistryPanel.preload();
      void LazyBusPanel.preload();
    });
  });

  return (
    <div
      class="host-root"
      classList={{
        "host-root--boot-ready": windowSystem.heavyPanelsVisible(),
      }}
    >
      <Frame
        title="Tauri Window System"
        meta={
          <span aria-live="polite">{windowSystem.heavyPanelsVisible() ? statusSummary() : "Booting shell"}</span>
        }
        actions={
          <button
            type="button"
            onClick={() => void windowSystem.refreshRegistry()}
            disabled={!windowSystem.canRefresh()}
            aria-label="Refresh registry snapshot"
            title="Refresh registry snapshot"
          >
            Refresh
          </button>
        }
        footer={<FooterPanel footerMessage={windowSystem.footerMessage} error={windowSystem.error} />}
      >
        <StatusPanel
          phase={windowSystem.phase}
          windowCount={windowSystem.windowCount}
          orphanCount={windowSystem.orphanCount}
        />
        <ControlBar
          selectedTargetLabel={windowSystem.selectedTargetLabel}
          canOpenChild={windowSystem.canOpenChild}
          canRequestSelected={windowSystem.canRequestSelected}
          canBroadcastStatus={windowSystem.canBroadcastStatus}
          canCloseSelected={windowSystem.canCloseSelected}
          onOpenChild={() => void windowSystem.openChild()}
          onRequestSelected={() => void windowSystem.pingChild()}
          onBroadcastStatus={() => void windowSystem.broadcastStatus()}
          onCloseSelected={() => void windowSystem.closeChild()}
        />
        <Show
          when={windowSystem.heavyPanelsVisible()}
          fallback={
            <div class="heavy-panels-skeleton">
              <section class="data-panel surface-shell" aria-label="registry snapshot">
                <header>
                  <h2>Registry snapshot</h2>
                  <p>Preparing the live registry view after the shell has painted.</p>
                </header>
                <p>Loading registry panel...</p>
              </section>
              <section class="data-panel surface-shell" aria-label="window bus activity">
                <header>
                  <h2>Window bus</h2>
                  <p>Preparing live message diagnostics after the shell has painted.</p>
                </header>
                <p>Loading bus panel...</p>
              </section>
            </div>
          }
        >
          <Suspense
            fallback={
              <div class="heavy-panels-skeleton">
                <section class="data-panel surface-shell" aria-label="registry snapshot">
                  <header>
                    <h2>Registry snapshot</h2>
                    <p>Loading registry chunk...</p>
                  </header>
                  <p>Warming registry panel...</p>
                </section>
                <section class="data-panel surface-shell" aria-label="window bus activity">
                  <header>
                    <h2>Window bus</h2>
                    <p>Loading bus chunk...</p>
                  </header>
                  <p>Warming bus panel...</p>
                </section>
              </div>
            }
          >
            <LazyRegistryPanel
              isRootWindow={isRootWindow}
              registryReady={windowSystem.registryReady}
              registryRows={windowSystem.registryRows}
              selectedTargetLabel={windowSystem.selectedTargetLabel}
              onSelectTarget={windowSystem.selectTarget}
            />
            <LazyBusPanel busMessage={windowSystem.busMessage} />
          </Suspense>
        </Show>
      </Frame>
    </div>
  );
}
