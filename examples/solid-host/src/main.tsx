import { render } from "solid-js/web";
import App from "./App";
import { markBoot, measureBoot, scheduleAfterFirstPaint } from "./bootTelemetry";
import "./styles.css";

markBoot("render start");

render(
  () => <App />,
  document.getElementById("root") as HTMLElement,
);

scheduleAfterFirstPaint(() => {
  markBoot("first paint");
  measureBoot("render -> first paint", "render start", "first paint");
});
