export function WindowFrame(props) {
    // This component only owns the structural shell.
    // Visual treatment stays in the host app so consumers can swap themes without touching the wrapper API.
    const titlebarProps = props.titlebarProps ?? {};
    const { class: titlebarClass, ...restTitlebarProps } = titlebarProps;
    return (<div class="window-frame">
      <div class={["window-frame__titlebar", titlebarClass].filter(Boolean).join(" ")} {...restTitlebarProps}>
        <div class="window-frame__titlebar-content">
          <div class="window-frame__title">{props.title ?? "Window System"}</div>
          {props.meta ? (<div class="window-frame__meta-shell">
              <div class="window-frame__meta">{props.meta}</div>
            </div>) : null}
          {props.actions ? (<div class="window-frame__actions">{props.actions}</div>) : null}
        </div>
      </div>
      <main class="window-frame__content">{props.children}</main>
      {props.footer ? (<footer class="window-frame__footer">{props.footer}</footer>) : null}
    </div>);
}
