import type { JSX } from "solid-js";
import { type WindowControlsProps, type WindowTitlebarProps } from "@tauri-controls/solid";
export interface WindowFrameProps {
    title?: JSX.Element;
    meta?: JSX.Element;
    actions?: JSX.Element;
    footer?: JSX.Element;
    titlebarProps?: Omit<WindowTitlebarProps, "controlsOrder" | "windowControlsProps">;
    windowControlsProps?: WindowControlsProps;
    children: JSX.Element;
}
export declare function WindowFrame(props: WindowFrameProps): JSX.Element;
