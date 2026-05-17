import { type JSX } from "solid-js";
export interface WindowFrameProps {
    title?: JSX.Element;
    meta?: JSX.Element;
    actions?: JSX.Element;
    footer?: JSX.Element;
    titlebarProps?: JSX.HTMLAttributes<HTMLDivElement>;
    children: JSX.Element;
}
export declare function WindowFrame(props: WindowFrameProps): JSX.Element;
