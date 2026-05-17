import type { Accessor } from "solid-js";

interface BusPanelProps {
  busMessage: Accessor<string>;
}

export default function BusPanel(props: BusPanelProps) {
  return (
    <section class="data-panel surface-shell" aria-label="window bus activity">
      <header>
        <h2>Window bus</h2>
        <p>Direct send, request/response, and broadcast all flow through the broker-backed helper.</p>
      </header>
      <p>{props.busMessage()}</p>
    </section>
  );
}
