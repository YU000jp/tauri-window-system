import { WindowTitlebar, } from "@tauri-controls/solid";
export function WindowFrame(props) {
    const titlebarProps = props.titlebarProps ?? {};
    const { class: titlebarClass, ...restTitlebarProps } = titlebarProps;
    const controlsProps = {
        justify: true,
        ...props.windowControlsProps,
    };
    return (<div class="window-frame">
      <WindowTitlebar controlsOrder="system" windowControlsProps={controlsProps} class={["window-frame__titlebar", titlebarClass].filter(Boolean).join(" ")} {...restTitlebarProps}>
        <div class="window-frame__titlebar-content">
          <div class="window-frame__drag-region">{props.title ?? "Window System"}</div>
          {props.meta ? (<div class="window-frame__meta-shell">
              <div class="window-frame__meta">{props.meta}</div>
            </div>) : null}
          {props.actions ? (<div class="window-frame__actions">{props.actions}</div>) : null}
        </div>
      </WindowTitlebar>
      <main class="window-frame__content">{props.children}</main>
      {props.footer ? (<footer class="window-frame__footer">{props.footer}</footer>) : null}
    </div>);
}
