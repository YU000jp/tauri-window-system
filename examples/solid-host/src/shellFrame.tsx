import type { JSX } from "solid-js";

export interface ShellFrameProps {
  title?: JSX.Element;
  meta?: JSX.Element;
  actions?: JSX.Element;
  footer?: JSX.Element;
  children: JSX.Element;
}

export function ShellFrame(props: ShellFrameProps) {
  return (
    <div class="window-frame">
      <div class="window-frame__titlebar">
        <div class="window-frame__titlebar-content">
          <div class="window-frame__title">{props.title ?? "Window System"}</div>
          {props.meta ? <div class="window-frame__meta-shell">{props.meta}</div> : null}
          {props.actions ? <div class="window-frame__actions">{props.actions}</div> : null}
        </div>
      </div>
      <main class="window-frame__content">{props.children}</main>
      {props.footer ? <footer class="window-frame__footer">{props.footer}</footer> : null}
    </div>
  );
}
